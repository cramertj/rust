//@ run-pass

#[provider(id = "wrapper_struct")]
type WrapperStruct;

fn main() {
    let x: WrapperStruct<i32> = WrapperStruct::<i32> { field_name: 10 };
    assert_eq!(x.field_name, 10);

    let x: WrapperStruct<String> = WrapperStruct::<String> { field_name: "foo".into() };
    assert_eq!(x.field_name, "foo");
}
