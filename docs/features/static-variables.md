# Static Variables

Static variables are variables whose lifetime is static, thus, are initialized on the creation of the code and live until it ends. They are NOT mutable due to problems with locks and synchronization, so if a static value needs to write, it MUST be a lock free instruction, beside that static variables have nothing different than other variables.

Static variables defined by extern blocks are defined on a library, runtime, or etc

```syx
static someValue: AtomicUint8 = AtomicUint8.new(); 
func main():void {
  let value = someValue.fetch_add(1); //this is possible due to atomics being atomics
}
```

Due to this limitation, static variables cannot have mutable references, nor be moved. # Static Variables

Static variables are variables with static lifetime. They are initialized during program initialization and remain alive until the program terminates.

Static variables cannot be mutated through ordinary mutable access, since doing so could introduce data races and would require synchronization between threads. If a static value needs to be modified, the operation must be performed through a mechanism that provides the required synchronization guarantees, such as an atomic or a lock-free operation.

Apart from their lifetime and access restrictions, static variables behave like other variables.

Static variables declared through `extern` blocks represent variables defined externally, such as by a library, runtime, or other external system.

```slynx
static someValue: AtomicUint8 = AtomicUint8.new();

func main(): void {
    let value = someValue.fetch_add(1);
}
```

The mutation in this example is allowed because `fetch_add` performs an atomic operation on the value rather than requiring ordinary mutable access.

Because static variables cannot have ordinary mutable access, they also cannot contain mutable references. Static variables cannot be moved either, since their lifetime is tied to the entire program rather than to a particular scope or owner.
