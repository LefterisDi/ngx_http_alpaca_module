use dom;
use parse;

use dom::{Map };
use morphing::{ MorphInfo};
use utils::{ get_html_img_inlined_format, content_to_c, c_string_to_str };

#[no_mangle]
pub extern "C" fn inline_css_content(pinfo: *mut MorphInfo, req_mapper: dom::Map) -> u8 {

    std::env::set_var("RUST_BACKTRACE", "full");

    let info = unsafe { &mut *pinfo };
    let uri  = c_string_to_str(info.uri).unwrap();

    let html = match c_string_to_str(info.content) {
        Ok (s) => s,
        Err(e) => {
            eprint!("libalpaca: cannot read html content of {}: {}\n", uri, e);
            return 0; // return NULL pointer if html cannot be converted to a string
        }
    };

    let document = parse::parse_html(html);

    // Vector of objects found in the html.
    parse::parse_css_and_inline(&document, req_mapper);

    let content = dom::serialize_html(&document);

    return content_to_c(content, info);
}

// Inserts the ALPaCA GET parameters to the html objects, and adds the fake objects to the html.
pub fn make_objects_inlined(objects: &mut Vec<dom::Object>, req_mapper : Map , n: usize) -> Result<(), String> {

    // Slice which contains initial objects
    let obj_for_inlining    = &objects[0..n];
    let mut objects_inlined = Vec::new();
    // let rest_obj = &objects[n..]; // Slice which contains ALPaCA objects

    for (i, object) in obj_for_inlining.iter().enumerate() {

        // Ignore objects without target size
        println!("OBJECT ITER {}", i);

        if object.target_size.is_none() {
            println!("OBJECT NO TARGET SIZE {}", object.uri);
        }

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

        // let path: String;

        // if attr != "style" {

        //     path = match node_get_attribute(node, attr) {
        //         Some(p) if p != "" && !p.starts_with("data:") => p,
        //         _ => continue,
        //     };

        // } else {
        //     path = object.uri.clone();
        // }

        // let temp = format!("{}/{}", root, path.as_str());
        let requested_uri = format!("/{}", object.uri);
        println!("URIRURI {}",requested_uri);
        let temp = get_html_img_inlined_format(req_mapper, &requested_uri);

        if attr != "style" {

            dom::node_set_attribute(node, attr, temp);
            objects_inlined.push(i);

        } else {
            //Replaces the <img src="q1.gif"> element for example with <img src="data:image/gif;charset=utf-8;base64 , ...">
            let last_child   = node.last_child().unwrap();
            let refc         = last_child.into_text_ref().unwrap();

            let mut refc_val = refc.borrow().clone();

            refc_val = refc_val.replace(&object.uri, &temp);

            *refc.borrow_mut() = refc_val;

            objects_inlined.push(i);
        }
    }

    for _ in objects_inlined.clone() {
        objects.remove( objects_inlined.pop().unwrap() );
    }

    Ok(())
}