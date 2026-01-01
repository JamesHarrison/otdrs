# TensorBoard Visualization Guide

## Overview

The OTDR ML training pipeline now includes comprehensive TensorBoard integration for visualizing model training dynamics, layer activations, and weights.

## Features

### 1. Training Metrics (Scalars)
- **Epoch-level metrics**: Loss, learning rate, and component losses
- **Batch-level metrics**: Real-time loss tracking during training
- **Validation metrics**: Validation loss and metrics per epoch

### 2. Weight & Gradient Histograms
- **Weight distributions**: All trainable parameters logged periodically
- **Gradient flow**: Gradient histograms to detect vanishing/exploding gradients
- **Layer-wise tracking**: Individual conv, transformer, and head weights

### 3. Activation Monitoring
- **Conv embedding activations**: Output distributions from 1D convolution layers
- **Transformer encoder activations**: Attention and feedforward layer outputs
- **Per-layer insights**: Track how activations evolve during training

### 4. Model Graph (experimental)
- Attempts to log the model computation graph
- Currently limited by dict-based model outputs
- Shows connections between layers when successful

## Usage

### Training with TensorBoard Logging

```bash
# Start training (logs automatically saved)
uv run python scripts/train.py \
  --train-dataset datasets/val_lmdb \
  --val-dataset datasets/val_lmdb \
  --output checkpoints \
  --epochs 100 \
  --batch-size 64

# Launch TensorBoard
tensorboard --logdir checkpoints/logs

# Open browser to http://localhost:6006
```

### Configuration Options

In `TrainingConfig`:

```python
config = TrainingConfig(
    # Enable/disable visualization features
    log_model_graph=True,          # Attempt graph logging
    log_histograms=True,            # Enable weight/activation histograms
    log_histograms_every=5,         # Log histograms every N epochs
    log_every=100,                  # Log batch metrics every N batches
)
```

### What to Look For

#### 1. **SCALARS Tab**
- **train/epoch_loss**: Overall training loss progression
- **train/lr**: Learning rate schedule
- **train/batch_***: Individual loss components (detection, position, measurements)
- **val/epoch_loss**: Validation loss (for overfitting detection)

#### 2. **HISTOGRAMS Tab**
Look for:
- **Centered distributions**: Weights should stay reasonably centered
- **Gradient magnitudes**: Should not approach zero (vanishing) or explode
- **Activation spread**: Conv/transformer outputs should have good variance

Warning signs:
- Weights drifting to extremes
- Gradients collapsing to zero
- Activations saturating (all 0s or 1s)

#### 3. **DISTRIBUTIONS Tab**
Alternative view of histograms showing evolution over time:
- **Smooth color gradients**: Healthy learning
- **Sudden color shifts**: Potential training instabilities
- **Collapsing distributions**: Possible dead neurons

#### 4. **GRAPHS Tab** (if loaded)
- Visual representation of model architecture
- Data flow through conv → transformer → heads
- Currently experimental due to dict outputs

## Advanced: Custom Logging

To add custom metrics in your training loop:

```python
# In trainer._train_epoch or custom training code
self.writer.add_scalar("custom/my_metric", value, global_step)
self.writer.add_histogram("custom/my_distribution", tensor, epoch)
```

## Troubleshooting

### Graph Not Showing
- Expected limitation with current architecture (dict outputs)
- Histograms and scalars will still work fine
- Future: Convert to NamedTuple outputs for full graph support

### High Memory Usage
- Reduce `log_histograms_every` (e.g., from 5 to 10)
- Histogram logging requires CPU memory for activation storage
- Consider disabling with `log_histograms=False` for very large models

### TensorBoard Not Updating
```bash
# Force refresh
tensorboard --logdir checkpoints/logs --reload_interval 5
```

## Performance Impact

- **Scalars**: Negligible (< 0.1% overhead)
- **Histograms**: ~1-2% overhead when logged (every N epochs)
- **Graph**: One-time cost at startup (~1-2 seconds)

Recommendation: Keep histograms enabled during development, optionally disable for long production runs.

## Next Steps

Future enhancements could include:
1. Attention weight heatmaps (visualize what the model focuses on)
2. Embedding projections (t-SNE/UMAP of learned representations)
3. Per-event-type metrics (separate loss tracking for splices, connectors, etc.)
4. Confusion matrices for event classification
5. Sample predictions visualization (trace + detected events overlay)
