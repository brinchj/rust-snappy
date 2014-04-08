#![feature(globs)]

#[link(name = "snappy",
       vers = "0.1.0",
       uuid = "17d57f36-462f-49c8-a3e1-109a7a4296c8",
       url = "https://github.com/thestinger/rust-snappy")]

#[comment = "snappy bindings"]
#[license = "MIT"]
#[crate_type = "lib"]

// For testing:
extern crate rand;
extern crate quickcheck;
// For runtime:
extern crate libc;

use libc::{c_int, size_t, c_void};
use std::rt::global_heap::malloc_raw;

use std::c_vec::{CVec};

#[link(name = "snappy")]
extern {
    fn snappy_compress(input: *u8,
                       input_length: size_t,
                       compressed: *mut u8,
                       compressed_length: *mut size_t) -> c_int;
    fn snappy_uncompress(compressed: *u8,
                         compressed_length: size_t,
                         uncompressed: *mut u8,
                         uncompressed_length: *mut size_t) -> c_int;
    fn snappy_max_compressed_length(source_length: size_t) -> size_t;
    fn snappy_uncompressed_length(compressed: *u8,
                                  compressed_length: size_t,
                                  result: *mut size_t) -> c_int;
    fn snappy_validate_compressed_buffer(compressed: *u8,
                                         compressed_length: size_t) -> c_int;
}

pub fn validate_compressed_buffer(src: &[u8]) -> bool {
    unsafe {
        snappy_validate_compressed_buffer(src.as_ptr(), src.len() as size_t) == 0
    }
}

fn malloc(n: uint) -> (*mut u8, ~CVec<u8>) {
    unsafe {
        assert!(n > 0);
        let mem = malloc_raw(n);
        let destroy = proc() { libc::free(mem as *mut c_void); };
        (mem, ~CVec::new_with_dtor(mem as *mut u8, n, destroy))
    }
}

pub fn compress(src: &[u8]) -> ~[u8] {
    unsafe {
        let srclen = src.len() as size_t;

        // Compute output size
        let mut dstlen = snappy_max_compressed_length(srclen);

        // Allocate output buffer
        let (pdst, vdst) = malloc(dstlen as uint);

        // Compress (should never fail)
        assert_eq!(0, snappy_compress(src.as_ptr(), srclen, pdst, &mut dstlen));

        vdst.as_slice().slice(0, dstlen as uint).into_owned()
    }
}

pub fn uncompress(src: &[u8]) -> Option<~[u8]> {
    unsafe {
        let srclen = src.len() as size_t;
        let mut dstlen: size_t = 0;

        // Check for validity and compute output size
        if snappy_uncompressed_length(src.as_ptr(), srclen, &mut dstlen) != 0 {
          return None;
        }

        // Output of zero length is either an compressed "" or invalid
        if dstlen == 0 {
          return if validate_compressed_buffer(src) {Some(~[])} else {None};
        }

        // Allocate output buffer
        let (pdst, vdst) = malloc(dstlen as uint);

        // Decompress and check result
        if snappy_uncompress(src.as_ptr(), srclen, pdst, &mut dstlen) != 0 {
          return None;  // SNAPPY_INVALID_INPUT
        }

        return Some(vdst.as_slice().slice(0, dstlen as uint).into_owned());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::task_rng;
    use quickcheck::{Config, Testable, gen};
    use quickcheck::{quickcheck_config};

    // QuickCheck configuration
    static SIZE: uint = 100;
    static CONFIG: Config = Config {
        tests: 100,
        max_tests: 1000,
    };

    // QuickCheck helpers:
    fn qcheck<A: Testable>(f: A) {
        quickcheck_config(CONFIG, &mut gen(task_rng(), SIZE), f)
    }

    #[test]
    fn qc_identity() {
        fn prop(data: Vec<u8>) -> bool {
            Some(data.as_slice().into_owned())
              == uncompress(compress(data.as_slice()))
        }
        qcheck(prop);
    }

    #[test]
    fn qc_compressed_data_validates() {
        fn prop(data: Vec<u8>) -> bool {
            validate_compressed_buffer(compress(data.as_slice()))
        }
        qcheck(prop);
    }

    #[test]
    fn qc_validate_agrees_with_uncompress() {
        fn prop(data: Vec<u8>) -> bool {
            let slice = data.as_slice();
            let uncompress_opt = uncompress(slice);
            if validate_compressed_buffer(slice) { uncompress_opt.is_some() }
            else { uncompress_opt.is_none() }
        }
        qcheck(prop);
    }

}