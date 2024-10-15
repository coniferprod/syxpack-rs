# SyxPack change log

## Version 0.17

* Updated and cleaned up dependencies.
* Added many manufacturers.
* Removed the KORG pack/unpack code, it belongs in its own crate.
* Removed the `read_file` function; use whatever you want/need.

## Version 0.12

* Depend on the [nybble](https://crates.io/crates/nybble) crate.
* Removed `Manufacturer::Development` special case.
* Removed binaries in favor of an upcoming binary crate that depends on syxpack (because you can't specify dependencies per binary)
* Fixed Universal SysEx definition (added target ID)
* The argument to the `read_file` function is now `&std::path::Path` instead of `&String`
