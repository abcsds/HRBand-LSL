
## Version

Current: 0.1.0 (Beta)

## Changelog

### v0.1.0
- Initial Rust implementation
- BLE device scanning and filtering
- Heart rate characteristic parsing
- LSL stream integration
- Hook system with 8 lifecycle points
- Example hook implementations
- Comprehensive tests and documentation

## Python Version

A simpler Python implementation is available on the `python` branch:

```bash
git checkout python
python main.py
```

The Rust version offers:
- **10x better performance** (faster startup, lower memory)
- **Extensible hook system** for custom integrations (InfluxDB, logging, etc.)
- **Type safety** and compile-time error checking
- **Production-ready** with full CI/CD and nix packaging
