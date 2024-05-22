
#[repr(C)]
struct Str{
    data: *const u8,
    len: usize,
}

extern "C"{ 
    fn vrmain(argc: i32, argv: *const Str) -> i32;
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
        fn print_$t(i: $t){
            print!("{}", i);
        }
    };
}

#[no_mangle]
extern "C" fn print_i128(i:i128){
    print!("{}", i);
}

#[no_mangle]
extern "C" fn print(s: Str) {
    let slice = unsafe { std::slice::from_raw_parts(s.data, s.len) };
    let string = std::str::from_utf8(slice).unwrap();
    print!("{}", string);
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
        vrmain(argc, args.as_ptr()) 
    }
}