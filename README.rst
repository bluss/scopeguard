
scopeguard
==========

Rust crate for a convenient RAII scope guard that will run a given closure when
it goes out of scope, even if the code between panics (assuming unwinding panic).


Please read the `API documentation here`__

__ http://bluss.github.io/scopeguard

|build_status|_ |crates|_

.. |build_status| image:: https://travis-ci.org/bluss/scopeguard.svg
.. _build_status: https://travis-ci.org/bluss/scopeguard

.. |crates| image:: http://meritbadge.herokuapp.com/scopeguard
.. _crates: https://crates.io/crates/scopeguard

How to use
----------

::

    #[macro_use(defer)] extern crate scopeguard;

    use scopeguard::guard;

    fn f() {
        defer!(println!("Called at return or panic"));
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

Recent Changes
--------------

- 0.2.0

  - Require Rust 1.6
  - Use `no_std` unconditionally
  - No other changes

- 0.1.2

  - Add macro ``defer!()``
