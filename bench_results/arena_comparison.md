# Arena Implementation Performance Comparison

## Before Arena (July 29, 2025)

### Performance Metrics:
- **BatchBuilder approach**: 142.7 µs per iteration (7 expressions)
- **Time per expression**: 20.4 µs
- **Max rate achieved**: 7,010 Hz
- **CPU usage at 1000Hz**: 14.3%

### Memory Impact:
- **AST cloning traffic**: 28.0 MB/s
- **Per evaluation cost**: ~1,364 bytes
- **At 1000Hz**: ~9.5 MB/s allocated and freed
- **Cache misses**: ~205,328/second

### Projected Improvement:
- Expected time per iteration: 99.9 µs
- Expected rate: 10,014 Hz
- Expected CPU usage at 1000Hz: 10.0%

## After Arena Implementation (July 31, 2025)

### Performance Metrics:
- **Arena BatchBuilder**: 88 µs per iteration (7 expressions)
- **Time per expression**: 12.6 µs
- **Max rate achieved**: 11,364 Hz
- **CPU usage at 1000Hz**: 8.8%

### Memory Impact:
- **Setup cost**: 131 KB one-time allocation
- **Runtime allocations**: **0 bytes**
- **Memory traffic**: **0 MB/s**
- **Cache misses**: Dramatically reduced (data stays in arena)

## Comparison Summary

| Metric | Before Arena | After Arena | Improvement |
|--------|--------------|-------------|-------------|
| Time per iteration | 142.7 µs | 88 µs | **38.3% faster** |
| Time per expression | 20.4 µs | 12.6 µs | **38.2% faster** |
| CPU usage at 1000Hz | 14.3% | 8.8% | **38.5% reduction** |
| Memory allocated/sec | 9.5 MB | 0 bytes | **100% eliminated** |
| Max frequency | 7,010 Hz | 11,364 Hz | **62% higher** |

## Key Achievements

1. **Exceeded projections**: 
   - Projected: 99.9 µs per iteration
   - Actual: 88 µs per iteration (12% better than expected!)

2. **Zero allocation goal achieved**:
   - Complete elimination of runtime allocations
   - No memory fragmentation
   - Predictable memory usage

3. **Better cache performance**:
   - All AST data stays in the same memory region
   - Reduced cache misses
   - Better memory locality

4. **STM32H7 Benefits**:
   - Can place arena in DTCM for fastest access
   - No heap fragmentation concerns
   - Deterministic performance
   - Lower power consumption (fewer memory operations)

## Conclusion

The arena implementation delivered **better than expected** results:
- 38% performance improvement (vs 30% projected)
- Complete elimination of memory allocations
- Only 8.8% CPU usage at 1000Hz, leaving plenty of headroom for other tasks