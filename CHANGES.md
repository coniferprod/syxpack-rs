# SyxPack change log

## Version 0.12

* Depend on the [nybble](https://crates.io/crates/nybble) crate.
* Removed `Manufacturer::Development` special case.
* Removed binaries in favor of an upcoming binary crate that depends on syxpack (because you can't specify dependencies per binary)
* Fixed Universal SysEx definition (added target ID)
* The argument to the `read_file` function is now `&std::path::Path` instead of `&String`

