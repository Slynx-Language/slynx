mod common;

use slynx_hir::SlynxHir;

fn load_object_methods_example(path: &str) {
    let _modules = common::load_example(path)
        .load_modules()
        .expect("modules should load (object-method resolution not yet implemented)");
}

#[test]
fn test_object_methods() {
    load_object_methods_example("examples/objMethod.syx");
}

#[test]
fn test_object_methods_with_multiple_methods() {
    load_object_methods_example("examples/objMethods.syx");
}

#[test]
fn test_object_static_methods() {
    load_object_methods_example("examples/objMethodStatic.syx");
}

#[test]
fn object_static_method_can_construct_empty_self() {
    let context = common::load_source(
        "pub object Empty {
             func new(): Self {
                 Self()
             }
         }
         func main(): void {
             let value = Empty.new();
         }",
    );
    let modules = context.load_modules().expect("modules should load");
    SlynxHir::new(&modules).expect("Self() should resolve to the current object type");
}
