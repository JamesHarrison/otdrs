# Training Data & Model Improvements

This document tracks identified issues and potential improvements for the OTDR ML event detection system.

## Known Issues

### 1. End-of-Fiber Events Not Predicted

**Root Cause**: The training data generation had a gap - when the config generator created simulations with "open" fiber ends (50% of cases), no event was generated at all. This meant:

- The model never saw labeled "end" events for open/cleaved fiber ends
- The only fiber terminations in training data were connectors at the fiber end

**Mitigations** (implemented in this codebase):
- Added synthetic "end" events in `labels.py` when fiber ends before range with no event nearby
- Added tail window coverage in `features.py` to ensure the fiber end is included in windowing
- Fixed event type count consistency between training and prediction

**✅ COMPLETED**: Updated `config_generator.py` to always create explicit end events:
- Added `EndType` enum with values: `cleaved`, `broken`, `terminated`
- Added `_generate_end_event()` helper function
- Modified `_generate_events()` to generate either a connector OR an explicit end event at fiber termination
- End type distribution: 50% connector, 50% non-connector (60% cleaved, 25% terminated, 15% broken)
- Added comprehensive tests for end event generation

**✅ COMPLETED**: Generated new training dataset with explicit end events:
- Location: `/home/james/otdrs/training_data/lmdb_v2`
- 500 simulations, 24,539 training windows
- End event distribution:
  - 50.2% connector ends
  - 31.4% cleaved ends
  - 11.0% terminated ends
  - 7.4% broken ends
- 100% coverage (every simulation has a termination event)

**Remaining Work**:

**Retrain the model** with the new dataset:

```bash
cd /home/james/otdrs/analysis
uv run python scripts/train.py \
    --train-dataset /home/james/otdrs/training_data/lmdb_v2 \
    --epochs 100 \
    --batch-size 64 \
    --output checkpoints/end_events_model
```

To generate a larger dataset for production training:

```bash
# Step 1: Generate more simulations (recommend 5000+ for production)
cd /home/james/otdrs/analysis
uv run python scripts/generate_simulations.py \
    --output-dir /path/to/large_simulations \
    --count 5000 \
    --threads 12

# Step 2: Convert to LMDB dataset
uv run python scripts/generate_lmdb_dataset.py \
    --input-dir /path/to/large_simulations \
    --output /path/to/large_lmdb_dataset
```

### 2. Event Type Count Configuration

The model's default `num_event_types=5` may not match the actual 6 event types in the data:
- connector
- splice
- macrobend
- splitter
- end
- fiber_transition

**Recommendation**: When training, explicitly set `num_event_types=6` in the training configuration.

### 3. Fiber Transition Events

Currently synthetic `FIBER_TRANSITION` events are added for segment boundaries where different fiber types meet. However:

- These are subtle events (mode field diameter changes)
- May be difficult to detect in practice
- Evaluation metrics should perhaps weight these lower

**Future Consideration**: Consider whether `fiber_transition` should remain a separate event type or be removed from training.

## Model Architecture Improvements

### 1. Per-Event Measurement Prediction

Current architecture predicts measurements globally per window. For windows with multiple events, this is suboptimal.

**Potential improvement**: Add an event-specific measurement head that outputs measurements per detected event position.

### 2. Confidence Calibration

The model's confidence scores may not be well-calibrated. Consider:
- Temperature scaling post-training
- Calibration dataset for threshold optimization
- Per-event-type threshold optimization

### 3. End-of-Fiber Detection Features

End-of-fiber events have distinctive characteristics:
- Sudden drop to noise floor after the event
- Characteristic Fresnel reflection pattern (for cleaved ends)
- Signal completely disappears (unlike other events)

**Potential feature additions**:
- Window-level feature indicating "signal drops to noise floor"
- Position relative to trace end

## Data Quality Improvements

### 1. Event Density Balancing

The current beta distribution for event placement creates crowded ends and sparse middles. Consider:
- More uniform event distribution for some portion of training data
- Explicit control over maximum event density per window

### 2. Noise Level Variation

Ensure training data includes a wide range of:
- Noise floor levels
- Dynamic ranges
- SNR conditions

### 3. Real-World Validation

Once real OTDR data is available:
- Create a validation set from real traces
- Fine-tune model on real data if needed
- Evaluate domain gap between simulated and real data

## Testing Improvements

### 1. End-of-Fiber Specific Tests

Add test cases specifically for:
- Open/cleaved fiber end detection
- Fiber end position accuracy
- Differentiation between connector-at-end vs open end

### 2. Edge Case Coverage

- Very short fibers (< 100m)
- Very long fibers (> 50km)
- High event density (> 5 events per window)
- Low SNR conditions
