rust-mps
==========
Rust bindings to the [Memory Pool System](http://www.ravenbrook.com/project/mps)

## Status
**NOTE:** This project is now inactive. It has serious bugs.

I no longer plan to use the MPS in [DuckLogic](https://ducklogic.org/).

This integration has proved difficult to debug with even the most trivial of examples.
This is mostly because the MPS [uses signals for memory barriers](https://www.ravenbrook.com/project/mps/master/manual/html/design/shield.html#overview).

Furthermore, it seems difficult to use with [zerogc](github.com/DuckLogic/zerogc).
This is primarily because the client cannot use safepoints to manage when garbage collection occurs.

In the future, it is possible that I will try integration with the MPS again.
If the performance prospects are good enough, I may even consider [paying Ravenbrook](https://www.ravenbrook.com/services/mm/)
to have them do the integration......

### Licensing
Now that the Memory Pool System has been re-licensed under the BSD 2-clause license,
it is possible to freely redistribute it with DuckLogic.

**NOTE**: I kept this private for a copule days in June 2021,
because I was exploring a proprietary integration with DuckLogic.
Rest assured any future integration will stay in the public.