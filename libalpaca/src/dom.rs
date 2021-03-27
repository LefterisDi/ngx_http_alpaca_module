//! Contains parsing routines
use html5ever::{ interface::QualName, LocalName, ns, namespace_url, serialize, serialize::{SerializeOpts} };
use kuchiki::NodeRef;
use std::{ str, ptr };
use std::ffi::CString;

// use std::os;
// use std::io::BufWriter;
// use std::io::Write;

// use std::io::prelude::*;
// use std::fs::File;

// Defines our basic object types, each of which has a corresponding
// unique (distribution, padding type) tuple.
#[derive(PartialEq)]
pub enum ObjectKind {
    FakeIMG,  // Fake alpaca image
    HTML   ,
    CSS    ,
    IMG    ,  // IMG: PNG, JPEG, etc.
	JS     ,
	CssImg ,
    Unknown,
}

// An object to be used in the morphing process.
pub struct Object {
    // Type of the Object
    pub kind: ObjectKind,
    // Content (Vector of bytes) of the Object
    pub content: Vec<u8>,
    // Node in the html
    pub node: Option<NodeRef>,
    // Size to pad the Object to
    pub target_size: Option<usize>,
    // The uri of the object, as mentioned in the html source
    pub uri: String,
}

#[repr(C)]
pub struct map {
    pub elems   : *mut *mut cell,
    pub capacity: libc::c_int   ,
    pub size    : libc::c_int   ,
}

#[repr(C)]
pub struct cell {
    pub next : *mut cell        ,
    pub value: *mut libc::c_void,
    pub key  : [libc::c_char; 0],
}

#[repr(C)]
pub struct RequestData {
    pub content: *mut libc::c_char,
    pub length : u32,
}

pub type Map = *mut map;

#[link(name = "map", kind = "static")]

extern "C" {
    // fn map_create() -> Map;
    // fn map_set(m: Map, key: *const libc::c_char, value: *mut libc::c_void);
    fn map_get(m: Map, key: *const libc::c_char) -> *mut libc::c_void;
}

impl Object {

    // Construct a real object from the html page
    pub fn existing(content: &[u8], kind: ObjectKind, uri: String, node: &NodeRef) -> Object {
        Object {
            kind        : kind                ,
            content     : content.to_vec()    ,
            node        : Some( node.clone() ),
            target_size : None                ,
            uri         : uri                 ,
        }
    }

    // Create padding object
    pub fn fake_image(target_size: usize) -> Object {
        Object {
            kind        : ObjectKind::FakeIMG       ,
            content     : Vec::new()                ,
            node        : None                      ,
            target_size : Some(target_size)         ,
            uri         : String::from("pad_object"),
        }
    }
}

pub fn get_map_element(req_mapper : Map , uri : String) -> Vec<u8> {

	let c_uri    = CString::new(uri).expect("CString::new Failed");

    let temp_old = unsafe { map_get(req_mapper, c_uri.as_ptr()) } as *mut RequestData;
    let temp_old = unsafe { &mut *temp_old };

	let mut element_data: Vec<u8> = Vec::new();
    element_data.reserve(temp_old.length as usize);

    unsafe {

        let dst_ptr = element_data.as_mut_ptr().offset(0) as *mut i8;

        ptr::copy_nonoverlapping( temp_old.content, dst_ptr, temp_old.length as usize );

        element_data.set_len( temp_old.length as usize );
    }
	element_data
}

pub fn create_element(name: &str) -> NodeRef {
    let qual_name = QualName::new( None, ns!(), LocalName::from(name) );
    NodeRef::new_element(qual_name, Vec::new())
}

pub fn create_css_node(css_text: &str) -> NodeRef {

	let elem_node = create_element("style");
	let css_text  = NodeRef::new_text(css_text);

    elem_node.append(css_text);
	elem_node
}

pub fn node_get_attribute(node: &NodeRef, name: &str) -> Option<String> {

    match node.as_element() {
        Some(element) => {
            match element.attributes.borrow().get(name) {
                Some(val) => Some( String::from(val) ),
                None      => None,
            }
        },
        None => None,
    }
}

pub fn node_set_attribute(node: &NodeRef, name: &str, value: String) {
    let elem = node.as_element().unwrap();
    elem.attributes.borrow_mut().insert(name, value);
}

pub fn serialize_html(dom: &NodeRef) -> Vec<u8> {

    let mut buf: Vec<u8> = Vec::new();
    let opts             = SerializeOpts::default();

    serialize(&mut buf, dom, opts).expect("serialization failed");

    buf
}

pub fn insert_empty_favicon(document: &NodeRef) {

    // Append the <link> either to the <head> tag, if exists, otherwise
    // to the whole document
    let node_data;  // to outlive the match
    let node = match document.select("head").unwrap().next() {
        Some(nd) => { node_data = nd; node_data.as_node() },
        None     => document                               ,
    };

	let elem = create_element("link");

    node_set_attribute( &elem, "href", String::from("data:,")       );
	node_set_attribute( &elem, "rel", String::from("shortcut icon") );

    node.append(elem);
}