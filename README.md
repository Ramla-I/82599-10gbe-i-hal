# Intel 82599 10GbE Intralingual HAL

An intralingual Hardware Abstraction Layer (HAL) for the Intel 82599 Ethernet Adapter. By intralingual we mean that all communication rules are conveyed to the compiler using the type system. To learn more about intralingual design refer to this [paper](https://dl.acm.org/doi/10.1145/3625275.3625398).

The key features of the i-HAL are:
1. a `struct` to represent the layout of memory-mapped I/O (MMIO) registers and packet descriptors, which can then be overlaid atop a region of memory.
2. Type wrappers on `struct` fields can enforce volatile access and read-only or write-only restrictions.
3. Only fields where every bit is accessible, without any restrictions, are publicly visible.
4. Fields marked as reserved or those with write restrictions have private visibility in the `struct`. The former are inaccessible outside the HAL, and the latter are only accessible through methods.
5. An `enum` or bitflag `struct` encodes the set of valid values for register writes forbidding arbitrary raw values at compile time.
6. Using linear types as a proof of work, we can statically enforce an order of operations between successive reads and writes of different registers or even between disjoint bitfields of the same register.

The i-HAL can be used in no-std crates, and is independent of the OS.
