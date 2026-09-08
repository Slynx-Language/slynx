# Extern values and types
Extern values are values/types that do not exist on the codebase directly via Slynx, but are instead referenced from the runtime being compiled to, or any other way that some content is provided.
For example, when compiling it to JS, there might be global objects such as `window`, `document`, `console`, etc. that slynx do not have by default, so, the idea is pretty simple, we simply say that 'an object named window
with type "Window" is available'. And that's it. That is how extern values work.

## Usage
Their definition on the codebase is not much different from the default values/types that are available on the codebase. In case, the unique difference is that the definition of the ones on the codebase can execute code, while the ones referenced from the runtime cannot, thus, they cannot be anything that contains a body. So a function might be able to be referenced, but not with a body.

They are defined using the `extern` keyword. Everything then that is inside an `extern` block is treated as a value/type definition that is extern, for example:

```slynx

extern {
  pub object Window {}
  pub object Document {}
  pub object Console {
    pub func log(&self, message: str);
  }
  static window: Window;
  static document: Document;
  static console: Console;
}

Which now makes it possible to execute `console.log("Hello, World!")`.
