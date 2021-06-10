use dom;
use parse;

use dom::Map;
use morphing::MorphInfo;
use utils::{ get_img_data_uri, content_to_c, c_string_to_str };

#[no_mangle]
pub extern "C" fn inline_all_css(pinfo: *mut MorphInfo, req_mapper: dom::Map) -> u8 {

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

    // Vector of objects found in the html
    parse::parse_css_and_inline(&document, req_mapper);

    let content = dom::serialize_html(&document);

    return content_to_c(content, info);
}

// Inserts the ALPaCA GET parameters to the html objects, and adds the fake objects to the html.
pub fn make_objects_inlined(objects: &mut Vec<dom::Object>, req_mapper: Map , n: usize, css_as_object: usize) -> Result<(), String> {

    // Slice which contains initial objects
    let mut objects_inlined = Vec::new();
    // let rest_obj = &objects[n..]; // Slice which contains ALPaCA objects

    let mut obj_cnt: usize = 0;

    for (i, object) in objects.iter().enumerate() {

        if obj_cnt == n {
            break;
        }

        // Ignore objects without target size
        println!("OBJECT ITER {}", i);

        if object.target_size.is_none() {
            println!("OBJECT NO TARGET SIZE {}", object.uri);
        }

        let node = object.node.as_ref().unwrap();

        let node_tag = node.as_element()
                           .unwrap()
                           .name
                           .local
                           .to_lowercase();

        let attr = match node_tag.as_ref() {
            "img" | "script" => "src",
            "link"           => "href",
            "style"          => "style",
            _                => panic!("shouldn't happen"),
        };

        if node_tag == "link" {

            if css_as_object == 0 {
                continue;
            }

            objects_inlined.push(i);

            let path = match dom::node_get_attribute(node, attr) {
                Some(p) if p != "" && !p.starts_with("data:") => p       ,
                _                                             => continue,
            };

            let res  = dom::get_map_element(req_mapper, format!("/{}", path) );

            let temp = res.iter().map(|&c| c as char).collect::<String>();

            let new_node = dom::create_css_node(&temp);

            node.insert_after(new_node);
            node.detach();

        } else if node_tag == "img" {

            objects_inlined.push(i);

            let requested_uri = format!("/{}", object.uri);
            let temp = get_img_data_uri(req_mapper, &requested_uri);

            dom::node_set_attribute(node, attr, temp);

        } else if node_tag == "style" {

            objects_inlined.push(i);

            let requested_uri = format!("/{}", object.uri);
            let temp = get_img_data_uri(req_mapper, &requested_uri);

            // Replaces the <img src="q1.gif"> element for example with <img src="data:image/gif;charset=utf-8;base64 , ...">
            let last_child   = node.last_child().unwrap();
            let refc         = last_child.into_text_ref().unwrap();

            let mut refc_val = refc.borrow().clone();

            refc_val = refc_val.replace(&object.uri, &temp);

            *refc.borrow_mut() = refc_val;
        }
        obj_cnt += 1;
    }

    for _ in objects_inlined.clone() {
        objects.remove( objects_inlined.pop().unwrap() );
    }

    Ok(())
}