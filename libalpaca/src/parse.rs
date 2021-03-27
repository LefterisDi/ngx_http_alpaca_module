use dom;
use utils;

use dom::{ ObjectKind, Object, Map };
use kuchiki::traits::*;
use kuchiki::{ parse_html_with_options, NodeRef, ParseOpts };
use std::str;


pub fn parse_html(input: &str) -> NodeRef {

    let mut opts = ParseOpts::default();
    opts.tree_builder.drop_doctype = true;

    let mut parser = parse_html_with_options(opts);
    parser.process(input.into());
    parser.finish()
}

pub fn parse_css_images(css_text: &str) -> Vec<String>{

	let mut images_paths: Vec<String> = Vec::new();

	if css_text.contains("url") {
		let spl_val: Vec<&str> = css_text.split("\n").collect();

		for item in spl_val {
			let mut new_it = utils::remove_whitespace(&item);

			if new_it.contains("url") {
                new_it = new_it.replace("\'", "\"");

                let spl       = new_it.split("url");
				let mut found = false;

                for it in spl {
					if found == true {

						let path = it.replace("\"", "").replace("(", "").replace(")", "").replace(")", "").replace(";", "");

                        if !path.contains("*/") {
							images_paths.push(path);
						}
						break;
					}
					found = true;
				}
			}
		}
	}
	return images_paths;
}

pub fn parse_css_names(document: &NodeRef) -> Vec<String> {

    // Objects vector
	let mut objects: Vec<String> = Vec::new();
	let mut found_favicon        = false;

    for node_data in document.select("link").unwrap() {

        let node = node_data.as_node();
		let name = node_data.name.local.to_lowercase();


		let path_attr = if name == "link" { "href" } else { "src" };
		let path      = match dom::node_get_attribute(node, path_attr) {
			Some(p) if p != "" && !p.starts_with("data:") => p       ,
			_                                             => continue,
		};

		println!("PATH {}",path);

		let temp = format!( "/{}", path.as_str());

		objects.push(temp);

		let rel  = dom::node_get_attribute(node, "rel").unwrap_or_default();
		match ( name.as_str(), rel.as_str() ) {
			("link", "stylesheet")                       => ObjectKind::CSS                          ,
			("link", "shortcut icon") | ("link", "icon") => { found_favicon = true; ObjectKind::IMG },
			_                                            => continue                                 ,
		};
	}

	// If no favicon was found, insert an empty one
	if !found_favicon {
		dom::insert_empty_favicon(document);
	}

    // objects.sort_unstable_by( |a, b| b.content.len().cmp( &a.content.len() ) ); // larger first
	objects
}

pub fn parse_css_and_inline(document: &NodeRef, req_mapper : Map) -> () {

	for node_data in document.select("link").unwrap() {

        let node      = node_data.as_node();
		let path_attr = "href";

        let path = match dom::node_get_attribute(node, path_attr) {
			Some(p) if p != "" && !p.starts_with("data:") => p       ,
			_                                             => continue,
		};

		if path.contains("favicon.ico") {
			continue;
		}

		let res  = dom::get_map_element(req_mapper, format!("/{}",path) );

		let par  = node.parent().unwrap();
		let temp = res.iter().map(|&c| c as char).collect::<String>();

        let new_node = dom::create_css_node(&temp);

        par.append(new_node);
	}

	let mut all_removed = false;

    while !all_removed {
		let mut removed = false;

        for node_data in document.select("link").unwrap() {

            let node      = node_data.as_node();
			let path_attr = "href";

			let path = match dom::node_get_attribute(node, path_attr) {
				Some(p) if p != "" && !p.starts_with("data:") => p       ,
				_                                             => continue,
			};

			if path.contains("favicon.ico") {
				continue;
			}
			node.detach();
			removed = true;
		}

		if !removed {
			all_removed = true;
		}
	}
}

// Parses the target size of an object from its HTTP request query.
// Returns 0 on error.
pub fn parse_target_size(query: &str) -> usize {

    let split1: Vec<&str> = query.split("alpaca-padding=").collect();
	let split2: Vec<&str> = split1[ split1.len() - 1 ].split("&").collect();
    let size_str          = split2[0];

	// Return the size
	match size_str.parse::<usize>() {
	  Ok(size) => return size,
	  Err(_)   => return 0
	}
}

// Parses the object's kind from its raw representation
pub fn parse_object_kind(mime: &str) -> ObjectKind {
	match mime {
		"text/html"                  => ObjectKind::HTML,
		"text/css"                   => ObjectKind::CSS ,
		x if x.starts_with("image/") => ObjectKind::IMG ,
    	_                            => ObjectKind::Unknown
    }
}

// Parses the objects contained in an HTML page.
pub fn parse_object_names(document: &NodeRef) -> Vec<String> {

    // Objects vector
	let mut objects: Vec<String> = Vec::new();
	let mut found_favicon        = false;

    for node_data in document.select("img,link,script").unwrap() {

        let node = node_data.as_node();
		let name = node_data.name.local.to_lowercase();

		let path_attr = if name == "link" { "href" } else { "src" };
		let path      = match dom::node_get_attribute(node, path_attr) {
			Some(p) if p != "" && !p.starts_with("data:") => p       ,
			_                                             => continue,
		};

		let temp = format!( "/{}", path.as_str());

		objects.push(temp.clone());

		let rel  = dom::node_get_attribute(node, "rel").unwrap_or_default();

        match ( name.as_str(), rel.as_str() ) {
			("link", "stylesheet")                       => ObjectKind::CSS                          ,
			("link", "shortcut icon") | ("link", "icon") => { found_favicon = true; ObjectKind::IMG },
			("script", _)                                => ObjectKind::JS                           ,
			("img", _)                                   => ObjectKind::IMG                          ,
			_                                            => continue                                 ,
		};
	}

	for node_data in document.select("style").unwrap() {

		let last_child   = node_data .as_node().last_child().unwrap();
		let refc         = last_child.into_text_ref().unwrap();

        let refc_val     = refc.borrow();
		let images_paths = parse_css_images(&refc_val);

		for img in images_paths {
			let temp = format!("/{}",img);
			objects.push(temp);
		}
	}

	// If no favicon was found, insert an empty one
	if !found_favicon {
		dom::insert_empty_favicon(document);
	}

    // objects.sort_unstable_by( |a, b| b.content.len().cmp( &a.content.len() ) ); // larger first
	objects
}

pub fn parse_objects(document: &NodeRef, req_mapper: Map) -> Vec<Object> {

    let mut objects: Vec<Object> = Vec::with_capacity(10);
	let mut found_favicon        = false;

    for node_data in document.select("img,link,script").unwrap() {

        let node = node_data.as_node();
		let name = node_data.name.local.to_lowercase();

		let path_attr = if name == "link" { "href" } else { "src" };
		let path      = match dom::node_get_attribute(node, path_attr) {
			Some(p) if p != "" && !p.starts_with("data:") => p       ,
			_                                             => continue,
		};

		let rel  = dom::node_get_attribute(node, "rel").unwrap_or_default();
		let kind = match ( name.as_str(), rel.as_str() ) {
			("link", "stylesheet")                       => ObjectKind::CSS                          ,
			("link", "shortcut icon") | ("link", "icon") => { found_favicon = true; ObjectKind::IMG },
			("script", _)                                => ObjectKind::JS                           ,
			("img", _)                                   => ObjectKind::IMG                          ,
			_                                            => continue                                 ,
		};

		/* Consider the posibility that the css file already has some GET parameters */
		let split: Vec<&str> = path.split('?').collect();
		let relative         = format!("/{}",split[0]);

		println!("REL {}",relative);

		let res = dom::get_map_element(req_mapper, relative);

		objects.push( Object::existing(&res, kind, path, node) );
	}

	for node_data in document.select("style").unwrap() {

		let node         = node_data .as_node();
		let last_child   = node_data .as_node().last_child().unwrap();
		let refc         = last_child.into_text_ref().unwrap();

        let refc_val     = refc.borrow();
		let images_paths = parse_css_images(&refc_val);

		for path in images_paths {

			let kind = ObjectKind::CSS;

			let split: Vec<&str> = path.split('?').collect();
			let relative         = format!("/{}",split[0]);

			let res = dom::get_map_element(req_mapper,relative);

			objects.push( Object::existing(&res, kind, path, node) );
		}
	}

	if !found_favicon {
		dom::insert_empty_favicon(document);
	}

    objects.sort_unstable_by( |a, b| b.content.len().cmp( &a.content.len() ) ); // larger first
	objects
}
