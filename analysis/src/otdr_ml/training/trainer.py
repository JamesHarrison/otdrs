"""Training loop and trainer class for OTDR models.

Optimized for CPU-first training with PyTorch.
"""

import json
import time
from collections import defaultdict
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

import torch
import torch.nn as nn
from torch.optim import AdamW
from torch.optim.lr_scheduler import CosineAnnealingLR
from torch.utils.data import DataLoader
from torch.utils.tensorboard import SummaryWriter
from tqdm.auto import tqdm

from .metrics import OTDRLoss, compute_metrics


@dataclass
class TrainingConfig:
    """Training configuration."""

    # Optimization
    learning_rate: float = 1e-4
    weight_decay: float = 0.01
    batch_size: int = 64
    epochs: int = 100

    # Learning rate schedule
    warmup_epochs: int = 5
    min_lr: float = 1e-6

    # Early stopping
    patience: int = 10
    min_delta: float = 1e-4

    # Checkpointing
    checkpoint_dir: Path = Path("checkpoints")
    save_every: int = 5  # Save every N epochs

    # Logging
    log_every: int = 100  # Log every N batches

    # Device
    device: str = "cpu"  # Start with CPU

    # DataLoader
    num_workers: int = 4

    # Loss weights
    loss_weights: dict[str, float] = field(
        default_factory=lambda: {
            "detection": 1.0,
            "position": 0.5,
            "loss_db": 0.3,
            "reflectance": 0.3,
        }
    )

    # Speed optimizations
    use_amp: bool = False
    compile_model: bool = False

    # Visualization
    log_model_graph: bool = True
    log_histograms: bool = True
    log_histograms_every: int = 5  # Log histograms every N epochs


class Trainer:
    """Trainer for OTDR models."""

    def __init__(
        self,
        model: nn.Module,
        config: TrainingConfig,
    ) -> None:
        """Initialize trainer.

        Args:
            model: Model to train.
            config: Training configuration.
        """
        self.model = model.to(config.device)
        self.config = config
        self.device = torch.device(config.device)

        # Loss function
        self.criterion = OTDRLoss(
            detection_weight=config.loss_weights["detection"],
            position_weight=config.loss_weights["position"],
            loss_db_weight=config.loss_weights["loss_db"],
            reflectance_weight=config.loss_weights["reflectance"],
        )

        # Optimizer
        self.optimizer = AdamW(
            model.parameters(),
            lr=config.learning_rate,
            weight_decay=config.weight_decay,
        )

        # Scheduler (will be set in train())
        self.scheduler = None

        # Training state
        self.epoch = 0
        self.global_step = 0
        self.best_loss = float("inf")
        self.patience_counter = 0
        self.history: list[dict[str, Any]] = []

        # Logging
        self.writer = SummaryWriter(log_dir=config.checkpoint_dir / "logs")

        # AMP Scaler - use proper API for PyTorch 2.x
        if config.use_amp:
            if config.device == "cuda":
                self.scaler = torch.cuda.amp.GradScaler(enabled=True)
            else:
                # CPU doesn't support GradScaler, disable AMP
                config.use_amp = False
                self.scaler = None
        else:
            self.scaler = None

        # Visualization hooks
        self.activation_storage = defaultdict(list)
        self._setup_visualization_hooks()

    def _setup_visualization_hooks(self) -> None:
        """Setup hooks for visualizing activations and weights."""

        # Hook for conv embedding layer outputs
        def conv_hook(module, input, output):
            if self.config.log_histograms and self.epoch % self.config.log_histograms_every == 0:
                self.activation_storage["conv_embed"].append(output.detach().cpu())

        # Hook for transformer encoder outputs
        def transformer_hook(module, input, output):
            if self.config.log_histograms and self.epoch % self.config.log_histograms_every == 0:
                self.activation_storage["transformer"].append(output.detach().cpu())

        # Register hooks on the model (check if modules exist as nn.Module, not Tensor)
        if hasattr(self.model, "conv_embed") and isinstance(
            getattr(self.model, "conv_embed"), nn.Module
        ):
            self.model.conv_embed.register_forward_hook(conv_hook)

        if hasattr(self.model, "transformer_encoder") and isinstance(
            getattr(self.model, "transformer_encoder"), nn.Module
        ):
            self.model.transformer_encoder.register_forward_hook(transformer_hook)

    def _log_model_graph(self, sample_input: torch.Tensor) -> None:
        """Log the model graph to TensorBoard.

        Args:
            sample_input: Sample input tensor for tracing.
        """
        try:
            # Wrap model to return just the tensor for graph tracing
            # (TensorBoard has issues with dict outputs)
            class GraphWrapper(nn.Module):
                def __init__(self, model):
                    super().__init__()
                    self.model = model

                def forward(self, x):
                    out = self.model(x)
                    # Return just one output for graph tracing
                    return out["event_logits"]

            wrapper = GraphWrapper(self.model)
            self.writer.add_graph(wrapper, sample_input)
            print(f"  Model graph logged to TensorBoard")
        except Exception as e:
            print(f"  Warning: Could not log model graph: {e}")

    def _log_histograms(self, epoch: int) -> None:
        """Log weight and activation histograms to TensorBoard.

        Args:
            epoch: Current epoch number.
        """
        # Log model weights and gradients
        for name, param in self.model.named_parameters():
            if param.requires_grad:
                # Log weights
                self.writer.add_histogram(f"weights/{name}", param.data, epoch)

                # Log gradients if they exist
                if param.grad is not None:
                    self.writer.add_histogram(f"gradients/{name}", param.grad, epoch)

        # Log activations that were captured
        for layer_name, activations in self.activation_storage.items():
            if activations:
                # Concatenate all batches and log
                combined = torch.cat(activations, dim=0)
                self.writer.add_histogram(f"activations/{layer_name}", combined, epoch)

        # Clear activation storage
        self.activation_storage.clear()

    def train(
        self,
        train_loader: DataLoader,
        val_loader: DataLoader | None = None,
        resume_from: Path | str | None = None,
    ) -> dict[str, Any]:
        """Train the model.

        Args:
            train_loader: Training data loader.
            val_loader: Optional validation data loader.
            resume_from: Optional checkpoint path to resume from.

        Returns:
            Training history.
        """
        # Setup scheduler
        torch.backends.cudnn.benchmark = True
        total_steps = len(train_loader) * self.config.epochs
        self.scheduler = CosineAnnealingLR(
            self.optimizer,
            T_max=total_steps,
            eta_min=self.config.min_lr,
        )

        # Create checkpoint directory
        self.config.checkpoint_dir.mkdir(parents=True, exist_ok=True)

        start_epoch = 0
        if resume_from:
            print(f"Resuming training from {resume_from}")
            self.load_checkpoint(resume_from)
            start_epoch = self.epoch + 1

        print(f"Training on {self.device}")
        print(f"Model parameters: {sum(p.numel() for p in self.model.parameters()):,}")
        print(f"TensorBoard logs: {self.config.checkpoint_dir / 'logs'}")

        # Log model graph on first epoch with a sample batch
        graph_logged = False

        try:
            for epoch in range(start_epoch, self.config.epochs):
                self.epoch = epoch

                # Training
                train_loss, train_metrics, sample_batch = self._train_epoch(train_loader)

                # Log model graph once with first sample
                if self.config.log_model_graph and not graph_logged and sample_batch is not None:
                    self._log_model_graph(sample_batch)
                    graph_logged = True

                # Log epoch metrics
                self.writer.add_scalar("train/epoch_loss", train_loss, epoch)
                for k, v in train_metrics.items():
                    self.writer.add_scalar(f"train/{k}", v, epoch)
                self.writer.add_scalar("train/lr", self.optimizer.param_groups[0]["lr"], epoch)

                # Log histograms periodically
                if self.config.log_histograms and (epoch % self.config.log_histograms_every == 0):
                    self._log_histograms(epoch)

                # Validation
                val_loss = None
                val_metrics = None
                if val_loader is not None:
                    val_loss, val_metrics = self._validate(val_loader)

                    self.writer.add_scalar("val/epoch_loss", val_loss, epoch)
                    for k, v in val_metrics.items():
                        self.writer.add_scalar(f"val/{k}", v, epoch)

                # Log epoch results
                epoch_result = {
                    "epoch": epoch,
                    "train_loss": train_loss,
                    "train_metrics": train_metrics,
                    "val_loss": val_loss,
                    "val_metrics": val_metrics,
                    "lr": self.optimizer.param_groups[0]["lr"],
                }
                self.history.append(epoch_result)

                # Checkpointing
                if (epoch + 1) % self.config.save_every == 0:
                    self._save_checkpoint(f"epoch_{epoch + 1}.pt")

                # Early stopping
                current_loss = val_loss if val_loss is not None else train_loss
                if current_loss < self.best_loss - self.config.min_delta:
                    self.best_loss = current_loss
                    self.patience_counter = 0
                    self._save_checkpoint("best.pt")
                else:
                    self.patience_counter += 1
                    if self.patience_counter >= self.config.patience:
                        print(f"Early stopping at epoch {epoch + 1}")
                        break

            # Save final checkpoint
            self._save_checkpoint("last.pt")

            # Save training history
            history_path = self.config.checkpoint_dir / "history.json"
            with open(history_path, "w") as f:
                json.dump(self.history, f, indent=2)

        finally:
            self.writer.close()

        return {"history": self.history, "best_loss": self.best_loss}

    def _train_epoch(
        self, loader: DataLoader
    ) -> tuple[float, dict[str, float], torch.Tensor | None]:
        """Train for one epoch.

        Returns:
            Tuple of (average loss, metrics dict, sample batch for visualization)
        """
        self.model.train()

        total_loss = 0.0
        all_metrics: dict[str, list[float]] = {}
        num_batches = 0
        sample_batch = None

        pbar = tqdm(loader, desc=f"Epoch {self.epoch + 1}/{self.config.epochs}")

        for batch_idx, batch in enumerate(pbar):
            # Save first batch for graph visualization
            if batch_idx == 0 and sample_batch is None:
                sample_batch = batch["features"][:1].to(self.device)

            # Move to device
            features = batch["features"].to(self.device)
            targets = {k: v.to(self.device) for k, v in batch.items() if k != "features"}

            # Forward pass
            self.optimizer.zero_grad(set_to_none=True)

            # Use autocast for mixed precision
            if self.config.use_amp and self.device.type == "cuda" and self.scaler is not None:
                with torch.cuda.amp.autocast():
                    outputs = self.model(features)
                    loss, loss_components = self.criterion(outputs, targets)

                # Backward with scaler
                self.scaler.scale(loss).backward()
                self.scaler.unscale_(self.optimizer)
                torch.nn.utils.clip_grad_norm_(self.model.parameters(), 1.0)
                self.scaler.step(self.optimizer)
                self.scaler.update()
            else:
                outputs = self.model(features)
                loss, loss_components = self.criterion(outputs, targets)

                # Standard backward
                loss.backward()
                torch.nn.utils.clip_grad_norm_(self.model.parameters(), 1.0)
                self.optimizer.step()

            if self.scheduler is not None:
                self.scheduler.step()

            total_loss += loss.item()
            num_batches += 1
            self.global_step += 1

            # Log batch metrics
            if self.global_step % self.config.log_every == 0:
                self.writer.add_scalar("train/batch_loss", loss.item(), self.global_step)
                for k, v in loss_components.items():
                    self.writer.add_scalar(f"train/batch_{k}", v, self.global_step)

            # Accumulate metrics
            for k, v in loss_components.items():
                if k not in all_metrics:
                    all_metrics[k] = []
                all_metrics[k].append(v)

            # Update progress bar
            pbar.set_postfix(
                {"loss": f"{loss.item():.4f}", "lr": f"{self.optimizer.param_groups[0]['lr']:.2e}"}
            )

        avg_loss = total_loss / num_batches
        avg_metrics = {k: sum(v) / len(v) for k, v in all_metrics.items()}

        return avg_loss, avg_metrics, sample_batch

    @torch.no_grad()
    def _validate(self, loader: DataLoader) -> tuple[float, dict[str, float]]:
        """Validate the model."""
        self.model.eval()

        total_loss = 0.0
        all_metrics: dict[str, list[float]] = {}
        num_batches = 0

        pbar = tqdm(loader, desc="Validation", leave=False)

        for batch in pbar:
            features = batch["features"].to(self.device)
            targets = {k: v.to(self.device) for k, v in batch.items() if k != "features"}

            outputs = self.model(features)
            loss, loss_components = self.criterion(outputs, targets)

            total_loss += loss.item()
            num_batches += 1

            for k, v in loss_components.items():
                if k not in all_metrics:
                    all_metrics[k] = []
                all_metrics[k].append(v)

            pbar.set_postfix({"loss": f"{loss.item():.4f}"})

        avg_loss = total_loss / num_batches
        avg_metrics = {k: sum(v) / len(v) for k, v in all_metrics.items()}

        return avg_loss, avg_metrics

    def _save_checkpoint(self, filename: str) -> None:
        """Save a checkpoint."""
        path = self.config.checkpoint_dir / filename
        checkpoint = {
            "epoch": self.epoch,
            "global_step": self.global_step,
            "model_state_dict": self.model.state_dict(),
            "optimizer_state_dict": self.optimizer.state_dict(),
            "best_loss": self.best_loss,
            "config": self.config,
            "scheduler_state_dict": self.scheduler.state_dict() if self.scheduler else None,
        }

        if self.scaler is not None:
            checkpoint["scaler_state_dict"] = self.scaler.state_dict()

        torch.save(checkpoint, path)

    def load_checkpoint(self, path: Path | str) -> None:
        """Load a checkpoint.

        Args:
            path: Path to checkpoint file.
        """
        # We save the full config object, so we need to enable pickle loading
        checkpoint = torch.load(path, map_location=self.device, weights_only=False)
        self.model.load_state_dict(checkpoint["model_state_dict"])
        self.optimizer.load_state_dict(checkpoint["optimizer_state_dict"])
        self.epoch = checkpoint["epoch"]
        self.global_step = checkpoint.get("global_step", 0)
        self.best_loss = checkpoint["best_loss"]

        if self.scheduler is not None and "scheduler_state_dict" in checkpoint:
            if checkpoint["scheduler_state_dict"] is not None:
                self.scheduler.load_state_dict(checkpoint["scheduler_state_dict"])

        if self.scaler is not None and "scaler_state_dict" in checkpoint:
            self.scaler.load_state_dict(checkpoint["scaler_state_dict"])
