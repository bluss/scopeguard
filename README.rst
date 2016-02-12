
scopeguard
==========

Rust crate for a convenient RAII scope guard.

Please read the `API documentation here`__

__ http://bluss.github.io/scopeguard

|build_status|_ |crates|_

.. |build_status| image:: https://travis-ci.org/bluss/scopeguard.svg
.. _build_status: https://travis-ci.org/bluss/scopeguard

.. |crates| image:: http://meritbadge.herokuapp.com/scopeguard
.. _crates: https://crates.io/crates/scopeguard

How to use
----------

```rust
extern crate scopeguard;

use scopeguard::guard;

fn f() {
    let _defer = guard((), |_| {
        println!("Called at return or panic");
    });
    panic!();
}

fn g() {
    let f = File::create("newfile.txt").unwrap();
    let mut file = guard(f, |f| {
        // write file at return or panic
        f.sync_all();
    });
    file.write("testme\n");
}
```
