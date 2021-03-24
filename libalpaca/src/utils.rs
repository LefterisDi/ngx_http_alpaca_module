use aux::stringify_error;
use base64;
use dom;
use kuchiki::NodeRef;
use libc;
use morphing::{MorphInfo};
use std::ffi::CStr;
use std::ffi::CString;
use std::fs;
use std::os::raw::c_int;


// -----------------------------------------------------------------------------------------------------
// REFERENCE MANIPULATION FUNCTIONS

// Appends the ALPaCA GET parameter to an html element
fn append_ref(object: &dom::Object) {

    // Construct the link with the appended new parameter
    let mut new_link = String::from("alpaca-padding=");

    new_link.push_str( &(object.target_size.unwrap().to_string()) ); // Append the target size

    let node = object.node.as_ref().unwrap();
    let attr = match node.as_element()
                         .unwrap()
                         .name
                         .local
                         .to_lowercase()
                         .as_ref()
    {
        "img" | "script" => "src",
        "link"           => "href",
        "style"          => "style",
        _                => panic!("shouldn't happen"),
    };

    // Check if there is already a GET parameter in the file path
    let prefix = if object.uri.contains("?") { '&' } else { '?' };

    new_link.insert    (0, prefix);
    new_link.insert_str(0, &object.uri);

    if attr != "style" {
        dom::node_set_attribute(node, attr, new_link);

    } else {

        let last_child   = node.last_child().unwrap();
        let refc         = last_child.into_text_ref().unwrap();

        let mut refc_val = refc.borrow().clone();

        refc_val = refc_val.replace(&object.uri, &new_link);

        *refc.borrow_mut() = refc_val;

        // println!("{}", refc.borrow());
    }
}

// Inserts the ALPaCA GET parameters to the html objects, and adds the fake objects to the html.
pub fn insert_objects_refs(document: &NodeRef, objects: &[dom::Object], n: usize) -> Result<(), String> {

    let init_obj    = &objects[0..n]; // Slice which contains initial objects
    let padding_obj = &objects[n..];  // Slice which contains ALPaCA objects

    println!("TO BE PADDED {}", n);

    for object in init_obj {
        // Ignore objects without target size
        if !object.target_size.is_none() {
            append_ref(&object);
        }
    }

    add_padding_objects(&document, padding_obj);

    Ok(())
}

// -----------------------------------------------------------------------------------------------------
// PADDING OBJECT ADDITION FUNCTION

// Adds the fake ALPaCA objects in the end of the html body
fn add_padding_objects(document: &NodeRef, objects: &[dom::Object]) {

    // Append the objects either to the <body> tag, if exists, otherwise
    // to the whole document
    let node_data; // to outlive the match
    let node = match document.select("body").unwrap().next() {
        Some(nd) => {
            node_data = nd;
            node_data.as_node()
        }
        None => document,
    };

    let mut i = 1;

    for object in objects {

        let elem = dom::create_element("img");

        dom::node_set_attribute(
            &elem,
            "src",
            format!(
                "/__alpaca_fake_image.png?alpaca-padding={}&i={}",
                object.target_size.unwrap(),
                i
            ),
        );
        dom::node_set_attribute( &elem, "style", String::from("visibility:hidden") );

        node.append(elem);
        i += 1;
    }
}

// -----------------------------------------------------------------------------------------------------
// CSS AND HTML FILE GETTER FUNCTIONS

#[no_mangle]
pub extern "C" fn get_html_required_files( pinfo: *mut MorphInfo, length: *mut c_int ) -> *mut *mut libc::c_char {
    let ptr = get_required_files(pinfo, length, true);
    ptr
}

#[no_mangle]
pub extern "C" fn get_required_css_files( pinfo: *mut MorphInfo, length: *mut c_int ) -> *mut *mut libc::c_char {
    let ptr = get_required_files(pinfo, length, false);
    ptr
}

#[no_mangle]
pub extern "C" fn get_required_files( pinfo: *mut MorphInfo, length: *mut c_int, is_html: bool ) -> *mut *mut libc::c_char {

    std::env::set_var("RUST_BACKTRACE", "full");

    let info = unsafe { &mut *pinfo };
    let uri  = c_string_to_str(info.uri).unwrap();

    // Convert arguments into &str
    let html = match c_string_to_str(info.content) {

        Ok (s) => s,
        Err(e) => {
            eprint!("libalpaca: cannot read html content of {}: {}\n", uri, e);
            return std::ptr::null_mut(); // return NULL pointer if html cannot be converted to a string
        }
    };

    let document    = dom::parse_html(html);

    let mut objects;

    if is_html {
        objects = dom::parse_object_names(&document); // Vector of objects found in the html.
    } else {
        objects = dom::parse_css_names(&document);    // Vector of objects found in the html.
    }

    let mut object_uris = vec![];

    for obj in &mut *objects {
        object_uris.push( CString::new( format!("{}", obj.to_owned()) ).unwrap() );
    }

    let mut out = object_uris.into_iter()
                             .map( |s| s.into_raw() )
                             .collect::< Vec<_> >();

    out.shrink_to_fit();

    let len = out.len();
    let ptr = out.as_mut_ptr();

    std::mem::forget(out);

    unsafe {
        std::ptr::write(length, len as c_int);
    }

    ptr
}

/*
    // #[no_mangle]
    // pub extern "C" fn get_html_required_files( pinfo: *mut MorphInfo, length: *mut c_int ) -> *mut *mut libc::c_char {

    //     std::env::set_var("RUST_BACKTRACE", "full");

    //     let info = unsafe { &mut *pinfo };
    //     let uri  = c_string_to_str(info.uri).unwrap();

    //     // Convert arguments into &str
    //     let html = match c_string_to_str(info.content) {

    //         Ok (s) => s,
    //         Err(e) => {
    //             eprint!("libalpaca: cannot read html content of {}: {}\n", uri, e);
    //             return std::ptr::null_mut(); // return NULL pointer if html cannot be converted to a string
    //         }
    //     };

    //     let document    = dom::parse_html(html);
    //     let mut objects = dom::parse_object_names(&document); // Vector of objects found in the html.

    //     let mut object_uris = vec![];

    //     for obj in &mut *objects {
    //         object_uris.push( CString::new( format!("{}", obj.to_owned()) ).unwrap() );
    //     }

    //     let mut out = object_uris.into_iter()
    //                              .map( |s| s.into_raw() )
    //                              .collect::< Vec<_> >();

    //     out.shrink_to_fit();

    //     let len = out.len();
    //     let ptr = out.as_mut_ptr();

    //     std::mem::forget(out);

    //     unsafe {
    //         std::ptr::write(length, len as c_int);
    //     }

    //     ptr
    // }

    // #[no_mangle]
    // pub extern "C" fn get_required_css_files( pinfo: *mut MorphInfo, length: *mut c_int ) -> *mut *mut libc::c_char {

    //     std::env::set_var("RUST_BACKTRACE", "full");

    //     let info = unsafe { &mut *pinfo };
    //     let uri  = c_string_to_str(info.uri).unwrap();

    //     // Convert arguments into &str
    //     let html = match c_string_to_str(info.content) {

    //         Ok (s) => s,
    //         Err(e) => {
    //             eprint!("libalpaca: cannot read html content of {}: {}\n", uri, e);
    //             return std::ptr::null_mut(); // return NULL pointer if html cannot be converted to a string
    //         }
    //     };

    //     let document    = dom::parse_html(html);
    //     let mut objects = dom::parse_css_names(&document); // Vector of objects found in the html.

    //     let mut object_uris = vec![];

    //     for obj in &mut *objects {
    //         object_uris.push( CString::new( format!("{}", obj.to_owned()) ).unwrap() );
    //     }

    //     let mut out = object_uris.into_iter()
    //                              .map( |s| s.into_raw() )
    //                              .collect::< Vec<_> >();

    //     out.shrink_to_fit();

    //     let len = out.len();
    //     let ptr = out.as_mut_ptr();

    //     std::mem::forget(out);

    //     unsafe {
    //         std::ptr::write(length, len as c_int);
    //     }

    //     ptr
    // }
*/

// -----------------------------------------------------------------------------------------------------
// FILE MANIPULATION AND DISCARDING


pub fn keep_local_objects(objects: &mut Vec< dom::Object >) {
    objects.retain( |obj| !obj.uri.contains("http:") && !obj.uri.contains("https:") )
}

pub fn get_file_extension(file_name: &String) -> String {
    let mut split: Vec<&str> = file_name.split(".").collect();
    split.pop().unwrap().to_owned()
}

pub fn get_img_format_and_ext(file_full_path: &String, file_name: &String) -> String {

    let base_img = fs::read(file_full_path).expect("Unable to read file");

    let extent = get_file_extension(&file_name);

    let ext: String;

    match extent.as_str() {
        "jpg" | "jpeg" => { ext = String::from("jpeg"); }
        "png"          => { ext = String::from("png");  }
        "gif"          => { ext = String::from("gif");  }
        _ => panic!("unknown image type"),
    };

    let res_base64 = base64::encode(&base_img);

    let temp = format!("data:image/{};charset=utf-8;base64,{}", ext, res_base64);

    temp
}

// -----------------------------------------------------------------------------------------------------
// CONTENT FROM RUST TO C AND VICE VERSA FUNCTIONS

// Builds the returned html, stores its size in html_size and returns a
// 'forgotten' unsafe pointer to the html, for returning to C
pub fn document_to_c(document: &NodeRef, info: &mut MorphInfo) -> u8 {
    let content = dom::serialize_html(document);
    return content_to_c(content, info);
}

pub fn content_to_c(content: Vec<u8>, info: &mut MorphInfo) -> u8 {

    info.size   = content.len();
    let mut buf = content.into_boxed_slice();

    info.content = buf.as_mut_ptr();
    std::mem::forget(buf);
    1
}

pub fn c_string_to_str<'a>(s: *const u8) -> Result<&'a str, String> {
    return stringify_error( unsafe { CStr::from_ptr(s as *const i8) }.to_str() );
}

// -----------------------------------------------------------------------------------------------------
// FREE MEMORY FUNCTION

// Frees memory allocated in rust.
#[no_mangle]
pub extern "C" fn free_memory(data: *mut u8, size: usize) {

    let s = unsafe { std::slice::from_raw_parts_mut(data, size) };
    let s = s.as_mut_ptr();

    unsafe {
        Box::from_raw(s);
    }
}