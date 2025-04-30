//@ run-pass

#[provider(id = "wrapper_struct")]
type WrapperStruct;

type Pi32 = WrapperStruct<i32>;
type PString = WrapperStruct<String>;

fn main() {
    let x: Pi32 = Pi32 { field_name: 10 };
    assert_eq!(x.field_name, 10);

    let x: PString = PString { field_name: "foo".into() };
    assert_eq!(x.field_name, "foo");
}
