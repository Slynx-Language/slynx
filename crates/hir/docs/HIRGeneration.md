# Hir Generation

The HIR generation is mainly lazy, this means that, the HIR will first try to find on the entry point of the codebase, the 'main' function. Then it will recursively find its dependants. Take for example the given code:

```syx

object Person {
  age: int,
  name: str
}

func add(a: int, b:int): int -> a + b;

func main(){
  let a = add(5,3);
}
```

What the HIR does is to parse this file, and then look for main. If main exists, it check it's dependencies on the signature. Since its signature doesn't have any types, it doesn't do anything until now.
Later it starts checking for the body. It finds the function call 'add' and then, does the same for 'add'. Since the 'object Person' wasn't referenced by main anywhere, it simply does not exist on the final HIR. 

The code should look somewhat like the following:

retrive_function_body(f){
  for statement in f.body {
    ...
  }
}

if let Some(f) = entry.find_function("main") {
  let signature = retrieve_function_signatures(f)?;
  let body = retrieve_function_body(f);
}

The resultant IR then should only contain function main and add. This is done internally by settings things on a queue. So anything that's got a body, its taken only the signature and it's registered, with its body being sent to a queue to be analyzed
when no other thing is ahead of it. So in that way, 'main' analyzes its full body, then enqueues anything it requires to work, getting only the signatures and id of the things it depends on. After so, the body of 'add' will be the next on the queue and it does the same, and so on.
