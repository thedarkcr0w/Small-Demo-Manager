# Compiler and runtime notices

The Windows GNU build uses the Rust standard library, GCC runtime components,
MinGW-w64 runtime, and winpthreads. Their notices are retained here separately
from the application's personal-use license.

* `rust/` contains the installed Rust toolchain's standard-library copyright
  document and license texts. Source: https://github.com/rust-lang/rust
* `gcc/` includes GPL license texts and `COPYING.RUNTIME`, the GCC Runtime
  Library Exception. The exception permits eligible non-GPL applications to use
  the covered runtime components without relicensing the independent application.
* `mingw-w64/` and `winpthreads/` contain the notices distributed with the compiler
  toolchain. The compiler itself is not included in the application ZIP.

GNU toolchain source / binary distribution:
https://github.com/niXman/mingw-builds-binaries/releases/tag/16.2.0-rt_v14-rev1

These files are attribution and licensing materials, not additional rights
claimed by darkcr0w over the toolchain or its authors' work.
