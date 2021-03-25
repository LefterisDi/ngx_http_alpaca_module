//! Contains main morphing routines.
use deterministic::*;
use distribution::{ sample_ge     ,
                    sample_ge_many,
                    sample_pair_ge,
                    Dist            };
use dom::{Map, Object, ObjectKind};
use dom;
use inlining::{ make_objects_inlined };
use kuchiki::NodeRef;
use pad::{ get_html_padding, get_object_padding };
use pad;
use utils::{ keep_local_objects ,
             document_to_c      ,
             content_to_c       ,
             c_string_to_str    ,
             insert_objects_refs };

// use image::gif::{GifDecoder, GifEncoder};
// use image::{ImageDecoder, AnimationDecoder};
// use std::fs::File;

#[repr(C)]
pub struct MorphInfo {
    // Request info
    alias                : usize    ,
    content_type         : *const u8,
    http_host            : *const u8,
    pub content          : *const u8, // u8 = uchar
    pub size             : usize    ,
    pub uri              : *const u8,
    query                : *const u8, // part after ?
    root                 : *const u8,

    // for probabilistic
    dist_html_size       : *const u8,
    dist_obj_num         : *const u8,
    dist_obj_size        : *const u8,
    probabilistic        : usize    , // boolean
    use_total_obj_size   : usize    ,

    // for deterministic
    max_obj_size         : usize    ,
    obj_num              : usize    ,
    obj_size             : usize    ,

    // for object inlining
    obj_inlining_enabled : bool     ,
}


#[no_mangle]
// It samples a new page using probabilistic morphing, changes
// the references to its objects accordingly, and pads it
pub extern "C" fn morph_html(pinfo: *mut MorphInfo, req_mapper: Map) -> u8 {

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

    let document = dom::parse_html(html);

    // Vector of objects found in the html
    let mut objects = dom::parse_html_objects_from_content(&document, req_mapper);

    keep_local_objects(&mut objects);

    // Number of original objects
    let mut orig_n = objects.len();

    let target_size = match if info.probabilistic != 0 {

        if info.obj_inlining_enabled == true {
            morph_probabilistic_with_inl( &document, &mut objects, &info, &mut orig_n )
        } else {
            morph_probabilistic( &document, &mut objects, &info )
        }

    } else {

        morph_deterministic( &document, &mut objects, &info, &mut orig_n )

    } {
        Ok (s) => s,
        Err(e) => {
            eprint!("libalpaca: cannot morph: {}\n", e);
            return document_to_c(&document, info);
        }
    };

    // Insert refs and add padding
    match insert_objects_refs(&document, &objects, orig_n) {

        Ok (_) => {}
        Err(e) => {
            eprint!("libalpaca: insert_objects_refs failed: {}\n", e);
            return document_to_c(&document, info);
        }
    }

    let mut content = dom::serialize_html(&document);

    // Pad the html to the target size.
    get_html_padding(&mut content, target_size);

    return content_to_c(content, info);
}

/*
    #[no_mangle]
    pub extern "C" fn morph_html(pinfo: *mut MorphInfo) -> u8 {

        std::env::set_var("RUST_BACKTRACE", "full");

        let info      = unsafe { &mut *pinfo };

        let root      = c_string_to_str(info.root)     .unwrap();
        let uri       = c_string_to_str(info.uri)      .unwrap();
        let http_host = c_string_to_str(info.http_host).unwrap();

        // Convert arguments into &str
        let html = match c_string_to_str(info.content) {

            Ok (s) => s,
            Err(e) => {
                eprint!("libalpaca: cannot read html content of {}: {}\n", uri, e);
                return 0; // return NULL pointer if html cannot be converted to a string
            }
        };

        let document  = dom::parse_html(html);

        let full_root = String::from(root).replace("$http_host", http_host);

        // Vector of objects found in the html
        let mut objects = dom::parse_objects(&document, full_root.as_str(), uri, info.alias);

        keep_local_objects(&mut objects);

        // Number of original objects
        let mut orig_n = objects.len();

        let target_size = match if info.probabilistic != 0 {

            if info.obj_inlining_enabled == true {
                morph_probabilistic_with_inl( &document, &mut objects, &info, &mut orig_n )
            } else {
                morph_probabilistic( &document, &mut objects, &info )
            }

        } else {

            if info.obj_inlining_enabled == true {
                morph_deterministic_with_inl( &document, &mut objects, &info, &mut orig_n )
            } else {
                morph_deterministic( &document, &mut objects, &info )
            }

        } {
            Ok (s) => s,
            Err(e) => {
                eprint!("libalpaca: cannot morph: {}\n", e);
                return document_to_c(&document, info);
            }
        };

        // Insert refs and add padding
        match insert_objects_refs(&document, &objects, orig_n) {

            Ok (_) => {}
            Err(e) => {
                eprint!("libalpaca: insert_objects_refs failed: {}\n", e);
                return document_to_c(&document, info);
            }
        }

        let mut content = dom::serialize_html(&document);

        // Pad the html to the target size.
        get_html_padding(&mut content, target_size);

        return content_to_c(content, info);
    }
*/

// Returns the object's padding.
#[no_mangle]
pub extern "C" fn morph_object(pinfo: *mut MorphInfo) -> u8 {

    let info = unsafe { &mut *pinfo };

    let content_type = c_string_to_str(info.content_type).unwrap();
    let query        = c_string_to_str(info.query)       .unwrap();

    let kind        = dom::parse_object_kind(content_type);
    let target_size = dom::parse_target_size(query);

    if (target_size == 0) || (target_size <= info.size) {
        // Target size has to be greater than current size.
        print!( "alpaca: morph_object: target_size ({}) cannot match current size ({})\n", target_size, info.size );
        return content_to_c(Vec::new(), info);
    }

    let padding = get_object_padding(kind, info.size, target_size); // Get the padding for the object.

    return content_to_c(padding, info);
}

fn morph_probabilistic_with_inl( document   : &NodeRef        ,
                                 objects    : &mut Vec<Object>,
                                 info       : &MorphInfo      ,
                                 new_orig_n : &mut usize       ) -> Result<usize, String>
{
    let dist_html_size = Dist::from( c_string_to_str(info.dist_html_size )? )?;
    let dist_obj_num   = Dist::from( c_string_to_str(info.dist_obj_num   )? )?;
    let dist_obj_size  = Dist::from( c_string_to_str(info.dist_obj_size  )? )?;

    // We'll have at least as many objects as the original ones
    let initial_obj_num = objects.len();

    // Sample target number of objects (count)
    let target_obj_num = match sample_ge(&dist_obj_num, 0) {

        Ok (c) => c,
        Err(e) => {
            eprint!(
                "libalpaca: could not sample object number ({}), leaving unchanged ({})\n",
                e, initial_obj_num
            );
            initial_obj_num
        }
    };

    let content = dom::serialize_html(&document);

    let final_obj_num: usize;
    let min_html_size: usize;

    if target_obj_num < initial_obj_num {

        final_obj_num = target_obj_num;
        min_html_size = content.len()
                        + 7                     // for the comment characters
                        + 23 * initial_obj_num; // for ?alpaca-padding=...
    } else {

        final_obj_num = target_obj_num - initial_obj_num;
        min_html_size = content.len()
                        + 7                     // for the comment characters
                        + 23 * initial_obj_num  // for ?alpaca-padding=...
                        + 94 * (final_obj_num); // for the fake images
    }

    let target_html_size;

    // Find object sizes
    if info.use_total_obj_size == 0 {

        // Sample each object size from dist_obj_size.
        target_html_size = sample_ge( &dist_html_size, min_html_size )?;

        // To more closely match the actual obj_size distribution, we'll sample values for all objects,
        // And then we'll use the largest to pad existing objects and the smallest for padding objects.
        let mut target_obj_sizes: Vec<usize>;

        if target_obj_num < initial_obj_num {
            target_obj_sizes = sample_ge_many( &dist_obj_size, 1, initial_obj_num )?;
        } else {
            target_obj_sizes = sample_ge_many( &dist_obj_size, 1, target_obj_num )?;
        }

        target_obj_sizes.sort_unstable(); // ascending

        // Pad existing objects
        for obj in &mut *objects {

            let needed_size = obj.content.len() + pad::min_obj_padding(&obj);

            // Take the largest size, if not enough draw a new one with this specific needed_size
            obj.target_size = if target_obj_sizes[target_obj_sizes.len() - 1] >= needed_size {
                Some(target_obj_sizes.pop().unwrap())

            } else {

                match sample_ge(&dist_obj_size, needed_size) {

                    Ok (size) => Some(size),
                    Err(e) => {
                        eprint!(
                            "libalpaca: warning: no padding was found for {} ({})\n",
                            obj.uri, e
                        );
                        None
                    }
                }
            };
        }

        if target_obj_num < initial_obj_num {

            let root      = c_string_to_str(info.root)     .unwrap();
            let http_host = c_string_to_str(info.http_host).unwrap();

            let full_root = String::from(root).replace("$http_host", http_host);

            // Insert refs and add padding
            make_objects_inlined( objects, full_root.as_str(), initial_obj_num - target_obj_num).unwrap();

            *new_orig_n = target_obj_num;

        } else {

            // Create padding objects, using the smallest of the sizes
            for i in 0..final_obj_num {
                objects.push(Object::fake_image(target_obj_sizes[i]));
            }
        }

    } else {
        // Sample the __total__ object size from dist_obj_size.

        // min size of all objects
        let min_obj_size = objects.into_iter()
                                  .map( |obj| obj.content.len() + pad::min_obj_padding(obj) )
                                  .sum();
        let target_obj_size;

        // Sample html/obj sizes, either together or separately
        if dist_obj_size.name == "Joint" {

            match sample_pair_ge( &dist_html_size, (min_html_size, min_obj_size) )? {
                (a, b) => {
                    target_html_size = a;
                    target_obj_size = b;
                }
            }

        } else {
            target_html_size = sample_ge( &dist_html_size, min_html_size )?;
            target_obj_size  = sample_ge( &dist_obj_size, min_obj_size   )?;
        }

        // create empty fake images
        // if target_obj_size > 0 && target_obj_num == 0 {
        //     // we chose a non-zero target_obj_size but have no objects to pad, create a fake one
        //     target_obj_num = 1;
        // }

        if target_obj_num < initial_obj_num {

            let root      = c_string_to_str(info.root)     .unwrap();
            let http_host = c_string_to_str(info.http_host).unwrap();

            let full_root = String::from(root).replace("$http_host", http_host);

            //insert refs and add padding
            make_objects_inlined( objects, full_root.as_str(), initial_obj_num - target_obj_num ).unwrap();

            *new_orig_n = target_obj_num;

        } else {

            // Create padding objects, using the smallest of the sizes
            for _ in 0..final_obj_num {
                objects.push(Object::fake_image(0));
            }
        }

        // Split all extra size equally among all objects
        let mut to_split = target_obj_size - min_obj_size;

        for (pos, obj) in objects.iter_mut().enumerate() {

            let pad = to_split / (target_obj_num - pos);

            obj.target_size = Some(obj.content.len() + pad::min_obj_padding(obj) + pad);
            to_split -= pad;
        }
    }

    Ok(target_html_size)
}

fn morph_deterministic( document   : &NodeRef        ,
                        objects    : &mut Vec<Object>,
                        info       : &MorphInfo      ,
                        new_orig_n : &mut usize       ) -> Result<usize, String>
{
    // We'll have at least as many objects as the original ones
    let initial_obj_no = objects.len();

    // Sample target number of objects (count) and target sizes for morphed
    // objects. Count is a multiple of "obj_num" and bigger than "min_count".
    // Target size for each objects is a multiple of "obj_size" and bigger
    // than the object's  original size.
    // let target_count = get_multiple(info.obj_num, initial_obj_no);

    let target_count;

    if info.obj_inlining_enabled {
        target_count = info.obj_num;
    } else {
        target_count = get_multiple(info.obj_num, initial_obj_no);
    }

    for i in 0..objects.len() {

        let min_size =   objects[i].content.len()
                       + match objects[i].kind {
                            ObjectKind::CSS | ObjectKind::JS => 4,
                            _ => 0,
                        };

        let obj_target_size  = get_multiple(info.obj_size, min_size);

        objects[i].target_size = Some(obj_target_size);
    }

    if target_count < initial_obj_no && info.obj_inlining_enabled {

        let root      = c_string_to_str(info.root).unwrap();
        let http_host = c_string_to_str(info.http_host).unwrap();

        let full_root = String::from(root).replace("$http_host", http_host);

        // Insert refs and add padding
        make_objects_inlined(objects, full_root.as_str(), initial_obj_no - target_count).unwrap();

        *new_orig_n = target_count;

    } else {

        let fake_objects_sizes: Vec<usize>;

        let fake_objects_count = target_count - initial_obj_no; // The number of fake objects

        // To get the target size of each fake object, sample uniformly a multiple
        // of "obj_size" which is smaller than "max_obj_size"
        fake_objects_sizes = get_multiples_in_range(info.obj_size, info.max_obj_size, fake_objects_count)?;

        // Add the fake objects to the vector
        for i in 0..fake_objects_count {
            objects.push( Object::fake_image(fake_objects_sizes[i]) );
        }
    }

    // Find target size,a multiple of "obj_size".
    let content = dom::serialize_html(&document);
    let html_min_size = content.len() + 7; // Plus 7 because of the comment characters.

    Ok(get_multiple(info.obj_size, html_min_size))
}

fn morph_probabilistic( document: &NodeRef        ,
                        objects : &mut Vec<Object>,
                        info    : &MorphInfo       ) -> Result<usize, String>
{
    let dist_html_size = Dist::from( c_string_to_str(info.dist_html_size )? )?;
    let dist_obj_num   = Dist::from( c_string_to_str(info.dist_obj_num   )? )?;
    let dist_obj_size  = Dist::from( c_string_to_str(info.dist_obj_size  )? )?;

    // We'll have at least as many objects as the original ones
    let initial_obj_num = objects.len();

    // Sample target number of objects (count)
    let mut target_obj_num = match sample_ge(&dist_obj_num, initial_obj_num) {

        Ok (c) => c,
        Err(e) => {
            eprint!(
                "libalpaca: could not sample object number ({}), leaving unchanged ({})\n",
                e, initial_obj_num
            );
            initial_obj_num
        }
    };

    // Sample target html size
    let content = dom::serialize_html(&document);
    let min_html_size = content.len()
                        + 7                                        // for the comment characters
                        + 23 * initial_obj_num                     // for ?alpaca-padding=...
                        + 94 * (target_obj_num - initial_obj_num); // for the fake images
    let target_html_size;

    // Find object sizes
    if info.use_total_obj_size == 0 {

        // Sample each object size from dist_obj_size.
        target_html_size = sample_ge(&dist_html_size, min_html_size)?;

        // To more closely match the actual obj_size distribution, we'll sample values for all objects,
        // And then we'll use the largest to pad existing objects and the smallest for padding objects.
        let mut target_obj_sizes: Vec<usize> = sample_ge_many(&dist_obj_size, 1, target_obj_num)?;

        target_obj_sizes.sort_unstable(); // ascending

        // Pad existing objects
        for obj in &mut *objects {
            let needed_size = obj.content.len() + pad::min_obj_padding(&obj);

            // Take the largest size, if not enough draw a new one with this specific needed_size
            obj.target_size = if target_obj_sizes[target_obj_sizes.len() - 1] >= needed_size {
                Some(target_obj_sizes.pop().unwrap())
            } else {
                match sample_ge(&dist_obj_size, needed_size) {
                    Ok(size) => Some(size),
                    Err(e) => {
                        eprint!(
                            "libalpaca: warning: no padding was found for {} ({})\n",
                            obj.uri, e
                        );
                        None
                    }
                }
            };
        }

        // create padding objects, using the smallest of the sizes
        for i in 0..target_obj_num - initial_obj_num {
            objects.push(Object::fake_image(target_obj_sizes[i]));
        }
    } else {
        // Sample the __total__ object size from dist_obj_size.
        // min size of all objects
        let min_obj_size = objects
            .into_iter()
            .map(|obj| obj.content.len() + pad::min_obj_padding(obj))
            .sum();
        let target_obj_size;

        // Sample html/obj sizes, either together or separately
        if dist_obj_size.name == "Joint" {
            match sample_pair_ge(&dist_html_size, (min_html_size, min_obj_size))? {
                (a, b) => {
                    target_html_size = a;
                    target_obj_size = b;
                }
            }
        } else {
            target_html_size = sample_ge(&dist_html_size, min_html_size)?;
            target_obj_size = sample_ge(&dist_obj_size, min_obj_size)?;
        }

        // Create empty fake images
        if target_obj_size > 0 && target_obj_num == 0 {
            // We chose a non-zero target_obj_size but have no objects to pad, create a fake one
            target_obj_num = 1;
        }

        for _ in 0..target_obj_num - initial_obj_num {
            objects.push(Object::fake_image(0));
        }

        // Split all extra size equally among all objects
        let mut to_split = target_obj_size - min_obj_size;

        for (pos, obj) in objects.iter_mut().enumerate() {
            let pad = to_split / (target_obj_num - pos);

            obj.target_size = Some(obj.content.len() + pad::min_obj_padding(obj) + pad);
            to_split -= pad;
        }
    }
    Ok(target_html_size)
}
