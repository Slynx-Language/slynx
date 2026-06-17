# Static Variables

Static variables are variables whose lifetime is static, thus, are initialized on the creation of the code and live until it ends. They are NOT mutable due to problems with locks and synchronization, so if a static value needs to write, it MUST be a lock free instruction, beside that static variables have nothing different than other variables.

Static variables defined by extern blocks are defined on a library, runtime, or etc

```syx
static someValue: AtomicUint8 = AtomicUint8.new(); 
func main():void {
  let value = someValue.fetch_add(1); //this is possible due to atomics being atomics
}
```
