use paste::paste;

#[repr(C)]
enum Ret{
    Ok,
    Err(Str)
}

#[repr(C)]
struct Str{
    data: *const u8,
    len: usize,
}

extern "C"{ 
    fn vrmain(argc: i32, argv: *const Str) -> Ret;
}
#[no_mangle]
extern "C" fn new_str(data: *const u8, len: usize) -> Str {
    Str {
        data: data,
        len: len,
    }
}

macro_rules! gen_print {
    ($t:ty) => {
        paste!{
            #[no_mangle]
            extern "C" fn [<print_$t>](i: $t){
                print!("{}", i);
            }
        }
    };
}

macro_rules! gen_println {
    ($t:ty) => {
        paste!{
            #[no_mangle]
            extern "C" fn [<println_$t>](i: $t){
                println!("{}", i);
            }
        }
    };
}

#[no_mangle]
extern "C" fn print_i128(i:i128){
    print!("{}", i);
}

#[no_mangle]
extern "C" fn vrka_Str_reverse(s: Str) -> *const Str{
    let slice = unsafe { std::slice::from_raw_parts(s.data, s.len) };
    let string = std::str::from_utf8(slice).unwrap();
    let reversed = string.chars().rev().collect::<String>();
    &new_str(reversed.as_ptr(), reversed.len())
}

#[no_mangle]
extern "C" fn print(s: Str) {
    let slice = unsafe { std::slice::from_raw_parts(s.data, s.len) };
    let string = std::str::from_utf8(slice).unwrap();
    print!("{}", string);
}

#[no_mangle]
extern "C" fn println(s: Str) {
    print!("raw string data:");
    for i in 0..s.len {
        unsafe {print!("{}", s.data.wrapping_add(i).read() as char);}
    }
    println!();
    let slice = unsafe { std::slice::from_raw_parts(s.data, s.len) };
    let string = std::str::from_utf8(slice).unwrap();
    println!("{}", string);
}

#[no_mangle]
extern "C" fn print_i(i:i64){
    print!("{}", i);
}

#[no_mangle]
extern "C" fn print_f(i:f64){
    print!("{}", i);
}

#[no_mangle]
extern "C" fn int_to_string(i:i64) -> Str {
    let s = i.to_string();
    new_str(s.as_ptr(), s.len())
}

// #[no_mangle]
// extern "C" fn print_bool(i:bool){
//     print!("{}", i);
// }
gen_print!(bool);
gen_println!(bool);
gen_println!(i128);

#[no_mangle]
extern "C" fn println_i(i:i64){
    println!("{}", i);
}

#[no_mangle]
extern "C" fn println_f(i:f64){
    println!("{}", i);
}

#[no_mangle]
extern "C" fn float_to_string(i:f64) -> Str {
    let s = i.to_string();
    new_str(s.as_ptr(), s.len())
}

#[no_mangle]
extern "C" fn main(argc: i32, argv: *const *const u8) -> i32{
    let args = unsafe { std::slice::from_raw_parts(argv, argc as usize) };
    let args = args.iter().map(|&arg| {
        let len = unsafe { libc::strlen(arg as *const i8) };
        new_str(arg, len as usize)
    }).collect::<Vec<_>>();
    unsafe {
        let out = vrmain(argc, args.as_ptr());
        match out {
            Ret::Ok => 0,
            Ret::Err(s) => {
                println(s);
                1
            }
        }
    }
}