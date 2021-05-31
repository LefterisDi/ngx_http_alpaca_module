#include <ngx_config.h>
#include <ngx_core.h>
#include <ngx_http.h>
#include <ngx_http_core_module.h>

#include <string.h>
#include <stdbool.h>

#include "./utils/map/map.h"

// This struct fills up from requests
// It's passed to rust
struct MorphInfo {

    // request info
    ngx_uint_t alias;
    u_char*    content_type;
    u_char*    http_host;
    u_char*    content;     // i8 = char

    ngx_uint_t size;
    u_char*    uri;
    u_char*    query;       // part after ?
    u_char*    root;

    // for probabilistic
    u_char*    dist_html_size;
    u_char*    dist_obj_num;
    u_char*    dist_obj_size;
    ngx_uint_t probabilistic;
    ngx_uint_t use_total_obj_size;

    // for deterministic
    ngx_uint_t max_obj_size;
    ngx_uint_t obj_num;
    ngx_uint_t obj_size;

    // for object inlining
    ngx_uint_t obj_inlining_enabled;
};

// This struct fills up from config
typedef struct {
    ngx_flag_t prob_enabled;
    ngx_flag_t deter_enabled;

    ngx_uint_t obj_num;
    ngx_uint_t obj_size;
    ngx_uint_t max_obj_size;

    ngx_str_t  dist_html_size;
    ngx_str_t  dist_obj_num;
    ngx_str_t  dist_obj_size;

    ngx_flag_t use_total_obj_size;
    ngx_flag_t obj_inlining_enabled;
    ngx_flag_t css_inlining_enabled;
} ngx_http_alpaca_loc_conf_t;

// Keep a state for each request
typedef struct {
    u_char*    response;
    u_char*    end;
    ngx_uint_t size;
    ngx_uint_t capacity;
} ngx_http_alpaca_ctx_t;

typedef struct {
	u_char*    content;
	u_int32_t  length;
} request_data;

// -----------------------------------------------------------------------------------------------------

u_char** get_html_required_files(struct MorphInfo *info , int *length);
u_char** get_required_css_files (struct MorphInfo *info , int *length);
u_char   inline_css_content     (struct MorphInfo *info , map req_mapper);
u_char   morph_html             (struct MorphInfo *info , map req_mapper);
u_char   morph_object           (struct MorphInfo *info);

void free_memory(u_char* data, ngx_uint_t size);

// -----------------------------------------------------------------------------------------------------

static ngx_int_t ngx_http_alpaca_header_filter  (ngx_http_request_t *r);
static ngx_int_t ngx_http_alpaca_body_filter    (ngx_http_request_t *r, ngx_chain_t *in);
static ngx_int_t ngx_http_alpaca_init           (ngx_conf_t *cf);

static void*     ngx_http_alpaca_create_loc_conf(ngx_conf_t *cf);
static char*     ngx_http_alpaca_merge_loc_conf (ngx_conf_t *cf, void *parent, void *child);

// -----------------------------------------------------------------------------------------------------

// These are the ALPaCA commands/directives given in the nginx.conf
static ngx_command_t ngx_http_alpaca_commands[] = {
    {
        ngx_string("alpaca_prob"),
        NGX_HTTP_MAIN_CONF | NGX_HTTP_SRV_CONF | NGX_HTTP_LOC_CONF | NGX_CONF_FLAG,
        ngx_conf_set_flag_slot, NGX_HTTP_LOC_CONF_OFFSET,
        offsetof(ngx_http_alpaca_loc_conf_t, prob_enabled), NULL
    },
    {
        ngx_string("alpaca_deter"),
        NGX_HTTP_MAIN_CONF | NGX_HTTP_SRV_CONF | NGX_HTTP_LOC_CONF | NGX_CONF_FLAG,
        ngx_conf_set_flag_slot, NGX_HTTP_LOC_CONF_OFFSET,
        offsetof(ngx_http_alpaca_loc_conf_t, deter_enabled), NULL
    },
    {
        ngx_string("alpaca_obj_num"),
        NGX_HTTP_MAIN_CONF | NGX_HTTP_SRV_CONF | NGX_HTTP_LOC_CONF | NGX_CONF_TAKE1,
        ngx_conf_set_num_slot, NGX_HTTP_LOC_CONF_OFFSET,
        offsetof(ngx_http_alpaca_loc_conf_t, obj_num), NULL
    },
    {
        ngx_string("alpaca_obj_size"),
        NGX_HTTP_MAIN_CONF | NGX_HTTP_SRV_CONF | NGX_HTTP_LOC_CONF | NGX_CONF_TAKE1,
        ngx_conf_set_num_slot, NGX_HTTP_LOC_CONF_OFFSET,
        offsetof(ngx_http_alpaca_loc_conf_t, obj_size), NULL
    },
    {
        ngx_string("alpaca_max_obj_size"),
        NGX_HTTP_MAIN_CONF | NGX_HTTP_SRV_CONF | NGX_HTTP_LOC_CONF | NGX_CONF_TAKE1,
        ngx_conf_set_num_slot, NGX_HTTP_LOC_CONF_OFFSET,
        offsetof(ngx_http_alpaca_loc_conf_t, max_obj_size), NULL
    },
    {
        ngx_string("alpaca_dist_html_size"),
        NGX_HTTP_MAIN_CONF | NGX_HTTP_SRV_CONF | NGX_HTTP_LOC_CONF | NGX_CONF_TAKE1,
        ngx_conf_set_str_slot, NGX_HTTP_LOC_CONF_OFFSET,
        offsetof(ngx_http_alpaca_loc_conf_t, dist_html_size), NULL
    },
    {
        ngx_string("alpaca_dist_obj_num"),
        NGX_HTTP_MAIN_CONF | NGX_HTTP_SRV_CONF | NGX_HTTP_LOC_CONF | NGX_CONF_TAKE1,
        ngx_conf_set_str_slot, NGX_HTTP_LOC_CONF_OFFSET,
        offsetof(ngx_http_alpaca_loc_conf_t, dist_obj_num), NULL
    },
    {
        ngx_string("alpaca_dist_obj_size"),
        NGX_HTTP_MAIN_CONF | NGX_HTTP_SRV_CONF | NGX_HTTP_LOC_CONF | NGX_CONF_TAKE1,
        ngx_conf_set_str_slot, NGX_HTTP_LOC_CONF_OFFSET,
        offsetof(ngx_http_alpaca_loc_conf_t, dist_obj_size), NULL
    },
    {
        ngx_string("alpaca_use_total_obj_size"),
        NGX_HTTP_MAIN_CONF | NGX_HTTP_SRV_CONF | NGX_HTTP_LOC_CONF | NGX_CONF_FLAG,
        ngx_conf_set_flag_slot, NGX_HTTP_LOC_CONF_OFFSET,
        offsetof(ngx_http_alpaca_loc_conf_t, use_total_obj_size), NULL
    },
    {
        ngx_string("alpaca_obj_inlining_enabled"),
        NGX_HTTP_MAIN_CONF | NGX_HTTP_SRV_CONF | NGX_HTTP_LOC_CONF | NGX_CONF_FLAG,
        ngx_conf_set_flag_slot, NGX_HTTP_LOC_CONF_OFFSET,
        offsetof(ngx_http_alpaca_loc_conf_t, obj_inlining_enabled), NULL
    },
    {
        ngx_string("alpaca_css_inlining_enabled"),
        NGX_HTTP_MAIN_CONF | NGX_HTTP_SRV_CONF | NGX_HTTP_LOC_CONF | NGX_CONF_FLAG,
        ngx_conf_set_flag_slot, NGX_HTTP_LOC_CONF_OFFSET,
        offsetof(ngx_http_alpaca_loc_conf_t, css_inlining_enabled), NULL
    },
    ngx_null_command
};

// -----------------------------------------------------------------------------------------------------

static ngx_http_module_t ngx_http_alpaca_module_ctx = {
    NULL,                 // preconfiguration
    ngx_http_alpaca_init, // postconfiguration

    NULL, // create main configuration
    NULL, // init main configuration

    NULL, // create server configuration
    NULL, // merge server configuration

    ngx_http_alpaca_create_loc_conf, // create location configuration
    ngx_http_alpaca_merge_loc_conf   // merge location configuration
};

ngx_module_t ngx_http_alpaca_module = {
    NGX_MODULE_V1,
    &ngx_http_alpaca_module_ctx,  // module context
    ngx_http_alpaca_commands,     // module directives
    NGX_HTTP_MODULE,              // module type
    NULL,                         // init master
    NULL,                         // init module
    NULL,                         // init process
    NULL,                         // init thread
    NULL,                         // exit thread
    NULL,                         // exit process
    NULL,                         // exit master
    NGX_MODULE_V1_PADDING
};

// next header and body filters in chain
static ngx_http_output_header_filter_pt ngx_http_next_header_filter;
static ngx_http_output_body_filter_pt   ngx_http_next_body_filter;

// -----------------------------------------------------------------------------------------------------

static ngx_int_t is_fake_image(ngx_http_request_t* r) {
    return ngx_strncmp(r->uri.data, "/__alpaca_fake_image.png", 24) == 0;
}

static ngx_int_t is_html(ngx_http_request_t* r) {
    // Note: Content-Type can contain a charset, eg "text/html; charset=utf-8"
    return ngx_strncmp(r->headers_out.content_type.data, "text/html", 9) == 0;
}

static ngx_int_t is_css(ngx_http_request_t* r) {
    // Note: Content-Type can contain a charset, eg "text/html; charset=utf-8"
    return ngx_strncmp(r->headers_out.content_type.data, "text/css", 8) == 0;
}

static ngx_int_t is_paddable(ngx_http_request_t* r) {

	return ( r->headers_out.content_type.len >= 6                                                                          &&
		     ngx_strncmp(r->headers_out.content_type.data, "image/", 6) == 0)                                              ||
		     ngx_strncmp(r->headers_out.content_type.data, "application/javascript", r->headers_out.content_type.len) == 0 ||
		     ngx_strncmp(r->headers_out.content_type.data, "text/css"              , r->headers_out.content_type.len) == 0;

		//    || ngx_strncmp(r->headers_out.content_type.data, "text/plain", r->headers_out.content_type.len) == 0;
}

// -----------------------------------------------------------------------------------------------------

static u_char* copy_ngx_str(ngx_str_t str, ngx_pool_t* pool) {

    u_char* res = ngx_pcalloc(pool, str.len + 1);

    ngx_memcpy(res, str.data, str.len);
    res[str.len] = '\0';

    return res;
}

static u_char* get_response(ngx_http_alpaca_ctx_t* ctx, ngx_http_request_t* r, ngx_chain_t* in , bool send) {

    u_char    *response;
    ngx_uint_t curr_chain_size = 0;

    for (ngx_chain_t *cl = in; cl; cl = cl->next)
        curr_chain_size += (cl->buf->last) - (cl->buf->pos);

    ctx->size += curr_chain_size;

    // Check if we need to allocate more space for the response
    if (ctx->size > ctx->capacity) {

        ctx->capacity = (2 * ctx->capacity > ctx->size) ? (2 * ctx->capacity)
                                                        : ctx->size;
        ctx->end      = ngx_pcalloc(r->pool, ctx->capacity + 1);

        u_char *start = ctx->end;

        ctx->end = ngx_copy(ctx->end, ctx->response, ctx->size - curr_chain_size);
        ngx_pfree(r->pool, ctx->response);

        ctx->response = start;
    }

    // Iterate through every buffer of the current chain and copy the contents
    for (ngx_chain_t *cl = in; cl; cl = cl->next) {

        ctx->end = ngx_copy( ctx->end, cl->buf->pos, (cl->buf->last) - (cl->buf->pos) );

        // If we reach the last buffer of the response, call ALPaCA
        if (cl->buf->last_in_chain) {

            *ctx->end = '\0';

            // Copy the padding and free the memory that was allocated in
            // rust using the custom "free memory" funtion.
            response = ngx_pcalloc( r->pool, (ctx->size + 1) * sizeof(u_char) );
            ngx_memcpy(response, ctx->response, ctx->size + 1);

            if (send == false) {
                strcpy( (char *)cl->buf->pos , "\0" );
                cl->buf->last = cl->buf->pos + 1;
            }
            return response;
        }

        if (send == false) {
            strcpy( (char *)cl->buf->pos , "\0" );
            cl->buf->last = cl->buf->pos + 1;
        }
    }
    return NULL;
}

struct MorphInfo* initialize_morph_html_struct(ngx_http_request_t         *r        ,
                                               ngx_http_core_loc_conf_t   *core_plcf,
                                               ngx_http_alpaca_loc_conf_t *plcf     ,
                                               ngx_http_alpaca_ctx_t      *ctx       ) {

    struct MorphInfo *main_info = NULL;

    main_info = malloc( sizeof(struct MorphInfo) );

    main_info->http_host = copy_ngx_str(r->headers_in.host->value, r->pool);
    main_info->root      = copy_ngx_str(core_plcf->root, r->pool);
    main_info->uri       = copy_ngx_str(r->uri, r->pool);


    main_info->alias     = core_plcf->alias != NGX_MAX_SIZE_T_VALUE ? core_plcf->alias : 0;
    main_info->content   = ctx->response;
    main_info->size      = ctx->size;

    main_info->dist_obj_size  = copy_ngx_str(plcf->dist_obj_size , r->pool);
    main_info->dist_obj_num   = copy_ngx_str(plcf->dist_obj_num  , r->pool);
    main_info->dist_html_size = copy_ngx_str(plcf->dist_html_size, r->pool);

    main_info->max_obj_size         = plcf->max_obj_size;
    main_info->obj_inlining_enabled = plcf->obj_inlining_enabled;
    main_info->obj_num              = plcf->obj_num;
    main_info->obj_size             = plcf->obj_size;
    main_info->probabilistic        = plcf->prob_enabled;
    main_info->use_total_obj_size   = plcf->use_total_obj_size;

    return main_info;
}

static ngx_int_t send_response(ngx_http_request_t* r, ngx_uint_t resp_size, u_char* response, ngx_chain_t* out, bool in_memory) {

    ngx_buf_t   *b;
    // ngx_chain_t  out;

    //Allocate buffer for the response
    b = ngx_calloc_buf(r->pool);

    if (b == NULL)
        return NGX_ERROR;

    //Copy the response to buffer
    b->pos  = response;
    b->last = b->pos + resp_size;

    b->last_buf      = 1;
    b->last_in_chain = 1;

    //saves content in read-only memory
    if (in_memory)
        b->memory = 1;

    out->buf  = b;
    out->next = NULL;

    return NGX_OK;
}

// -----------------------------------------------------------------------------------------------------

static void* ngx_http_alpaca_create_loc_conf(ngx_conf_t* cf) {

    ngx_http_alpaca_loc_conf_t* conf;

    conf = ngx_pcalloc( cf->pool, sizeof(ngx_http_alpaca_loc_conf_t) );

    if (conf == NULL)
        return NULL;

    conf->prob_enabled         = NGX_CONF_UNSET;
    conf->deter_enabled        = NGX_CONF_UNSET;
    conf->obj_num              = NGX_CONF_UNSET_UINT;
    conf->obj_size             = NGX_CONF_UNSET_UINT;
    conf->max_obj_size         = NGX_CONF_UNSET_UINT;
    conf->use_total_obj_size   = NGX_CONF_UNSET;
    conf->obj_inlining_enabled = NGX_CONF_UNSET;
    conf->css_inlining_enabled = NGX_CONF_UNSET;

    return conf;
}

static char* ngx_http_alpaca_merge_loc_conf(ngx_conf_t* cf, void* parent, void* child) {

    ngx_http_alpaca_loc_conf_t* prev = parent;
    ngx_http_alpaca_loc_conf_t* conf = child;

    ngx_conf_merge_value     (conf->prob_enabled        , prev->prob_enabled        , 0 );
    ngx_conf_merge_value     (conf->deter_enabled       , prev->deter_enabled       , 0 );
    ngx_conf_merge_uint_value(conf->obj_num             , prev->obj_num             , 0 );
    ngx_conf_merge_uint_value(conf->obj_size            , prev->obj_size            , 0 );
    ngx_conf_merge_uint_value(conf->max_obj_size        , prev->max_obj_size        , 0 );
    ngx_conf_merge_str_value (conf->dist_html_size      , prev->dist_html_size      , "");
    ngx_conf_merge_str_value (conf->dist_obj_num        , prev->dist_obj_num        , "");
    ngx_conf_merge_str_value (conf->dist_obj_size       , prev->dist_obj_size       , "");
    ngx_conf_merge_value     (conf->use_total_obj_size  , prev->use_total_obj_size  , 0 );
    ngx_conf_merge_value     (conf->obj_inlining_enabled, prev->obj_inlining_enabled, 0 );
    ngx_conf_merge_value     (conf->css_inlining_enabled, prev->css_inlining_enabled, 0 );


    // Check if the directives' arguments are properly set
    if ( (conf->prob_enabled && conf->deter_enabled) ) {
        ngx_conf_log_error( NGX_LOG_EMERG, cf, 0, "Both probabilistic and deterministic ALPaCA are enabled." );
        return NGX_CONF_ERROR;
    }

    if (conf->prob_enabled && conf->dist_obj_size.len == 0) {
        ngx_conf_log_error( NGX_LOG_EMERG, cf, 0, "dist_obj_size is needed in probabilistic mode" );
        return NGX_CONF_ERROR;
    }

    if (conf->deter_enabled) {

        if ( (conf->obj_size <= 0) || (conf->max_obj_size <= 0) ) {
            ngx_conf_log_error(NGX_LOG_EMERG, cf, 0, "You can't provide non-positive values or no "
                                                     "values at all for deterministic ALPaCA."      );
            return NGX_CONF_ERROR;
        }

        if ( conf->max_obj_size < conf->obj_size ) {
            ngx_conf_log_error(NGX_LOG_EMERG, cf, 0, "Object size cannot be greater than max object "
                                                     "size for deterministic ALPaCA."                 );
            return NGX_CONF_ERROR;
        }

        if ( conf->max_obj_size % conf->obj_size != 0 ) {
            ngx_conf_log_error(NGX_LOG_EMERG, cf, 0, "Max object size has to be a multiple of object "
                                                     "size for deterministic ALPaCA."                  );
            return NGX_CONF_ERROR;
        }
    }
    return NGX_CONF_OK;
}

static ngx_int_t ngx_http_alpaca_init(ngx_conf_t* cf) {

    // Install handler in header filter chain
    ngx_http_next_header_filter = ngx_http_top_header_filter;
    ngx_http_top_header_filter  = ngx_http_alpaca_header_filter;

    // Install handler in body filter chain
    ngx_http_next_body_filter = ngx_http_top_body_filter;
    ngx_http_top_body_filter  = ngx_http_alpaca_body_filter;

    return NGX_OK;
}

// -----------------------------------------------------------------------------------------------------

int8_t execute_subrequests( struct MorphInfo          **main_info ,
                            map                        *req_mapper,
                            int                        *subreq_tbd,
                            ngx_chain_t                *in        ,
                            ngx_http_alpaca_ctx_t      *ctx       ,
                            ngx_http_alpaca_loc_conf_t *plcf      ,
                            ngx_http_core_loc_conf_t   *core_plcf ,
                            ngx_http_request_t         *r           )
{
    ngx_chain_t *cl;

    *req_mapper   = NULL;
    *main_info    = NULL;
    *subreq_tbd   = 0;

    int rc                 = NGX_OK;
    u_char **objects       = NULL;
    ngx_http_request_t *sr = NULL;

    // Cycle through the buffer chain and set the to prepare the
    // chain for the subrequest buffers to come
    for (cl = in; cl; cl = cl->next) {
        if (cl->buf->last_buf) {
            cl->buf->last_buf      = 0;
            cl->buf->last_in_chain = 1;
        }
    }

    // -------------------------------------------------------------------------------------

    *main_info = initialize_morph_html_struct(r, core_plcf, plcf, ctx);

    // Collects required css filenames. If not activated or no files were found
    // then we get every other filename that can be padded inside the given html
    if (plcf->css_inlining_enabled) {
        objects = get_required_css_files(*main_info, subreq_tbd);
    }

    if (*subreq_tbd == 0)
        objects = get_html_required_files(*main_info, subreq_tbd);

    if (*req_mapper == NULL) {
        *req_mapper = map_create();
        if (*req_mapper == NULL){
            printf("ERROR REQ CONT MAPPER\n");
            return NGX_ERROR;
        }
    }

    // -------------------------------------------------------------------------------------
    // Do subrequests one for each filename found and contained in objects array

    for (int i = 0; rc == NGX_OK && i < *subreq_tbd ; i++) {

        ngx_str_t uri;

        (&uri)->len  = strlen( (const char *)objects[i] );
        (&uri)->data = (u_char *)objects[i];

        ngx_http_subrequest(r, &uri , NULL , &sr, NULL, 0);
    }

    if ( *subreq_tbd == 0 )
        return 0;

    return 1;
}

void execute_html_object_subrequests( struct MorphInfo   *main_info ,
                                      int                *subreq_tbd,
                                      ngx_http_request_t *r           )
{
    int rc = NGX_OK;
    u_char **objects = NULL;
    ngx_http_request_t *sr = NULL;

    objects = get_html_required_files(main_info , subreq_tbd);

    // Do subrequests for all HTML files
    for (int i=0 ; rc == NGX_OK && i < *subreq_tbd ; i++) {

        ngx_str_t uri;

        (&uri)->len  = strlen( (const char *)objects[i] );
        (&uri)->data = (u_char *)objects[i];

        ngx_http_subrequest(r, &uri , NULL, &sr, NULL, 0);
    }
}

void simple_html_morph( struct MorphInfo       *main_info ,
                        map                     req_mapper,
                        u_char                **response  ,
                        ngx_http_alpaca_ctx_t  *ctx       ,
                        ngx_http_request_t     *r           )
{
    // Pad html body
    if ( morph_html(main_info, req_mapper) ) {

        // Copy the morphed html and free the memory that was
        // allocated in rust using the custom "free memory" funtion
        *response = ngx_pcalloc( r->pool, main_info->size * sizeof(u_char) );

        ngx_memcpy(*response, main_info->content, main_info->size);
        ngx_pfree (r->pool, ctx->response);

        free_memory(main_info->content, main_info->size);

        ctx->size = main_info->size;

    } else {

        // Alpaca failed. This might happen if the content was not
        // really html, eg it was proxied from some upstream server
        // that returned gziped content. We log this and return the
        // original content
        ngx_log_error( NGX_LOG_ERR                                            ,
                       r->connection->log                                     ,
                       0                                                      ,
                       "[Alpaca filter]: could not process html content. If "
                       "you use proxy_pass, set proxy_set_header "
                       "Accept-Encoding \"\" so that the upstream server "
                       "returns raw html, "
                     );

        *response = ctx->response;
    }
}

bool pad_object( u_char                **response     ,
                 ngx_uint_t             *response_size,
                 ngx_http_alpaca_ctx_t  *ctx          ,
                 ngx_http_request_t     *r              )
{

    // Call ALPaCA to get the padding
    struct MorphInfo info = {
        .content_type = copy_ngx_str(r->headers_out.content_type, r->pool),
        .query        = copy_ngx_str(r->args, r->pool),
        .size         = ctx->size,
    };

    // Get corresponding content for specific file
    // and pass it to morph_object
    if ( !morph_object(&info) )
        return false;

    // Copy the padding and free the memory that was allocated in
    // rust using the custom "free memory" funtion
    *response = ngx_pcalloc( r->pool, (info.size) * sizeof(u_char) );

    ngx_memcpy(*response, info.content, info.size);
    free_memory(info.content, info.size);

    *response_size = info.size;

    return true;
}

void map_insert_response( map                    req_mapper,
                          u_char                *response  ,
                          ngx_http_alpaca_ctx_t *ctx       ,
                          ngx_http_request_t    *r           )
{
    request_data* req_data = malloc(sizeof(request_data));
    req_data->content      = malloc(ctx->size);

    memset(req_data->content, 0, ctx->size);
    memcpy(req_data->content, response, ctx->size);

    req_data->length = ctx->size;

    map_set(req_mapper, (char *)r->uri.data, req_data);
}

void process_html_objects( map                     req_mapper,
                           struct MorphInfo       *main_info ,
                           u_char                **response  ,
                           ngx_http_alpaca_ctx_t  *ctx       ,
                           ngx_http_request_t     *r           )
{
    // Copy the initial response of the html to a string in order to return it in case of error
    u_char* init_response = ngx_pcalloc(r->pool , main_info->size * sizeof(u_char));
    memset( (char *)init_response , 0, main_info->size);
    memcpy( (char *)init_response, (char *)main_info->content, main_info->size );

    if ( morph_html(main_info, req_mapper) ) {

        // Copy the morphed html and free the memory that was
        // allocated in rust using the custom "free memory" funtion
        *response = ngx_pcalloc( r->pool, main_info->size * sizeof(u_char) );

        ngx_memcpy(*response, main_info->content, main_info->size);
        ngx_pfree (r->pool, ctx->response);

        free_memory(main_info->content, main_info->size);

        ctx->size = main_info->size;

    } else {

        // Alpaca failed. This might happen if the content was not
        // really html, eg it was proxied from some upstream server
        // that returned gziped content. We log this and return the
        // original content
        ngx_log_error( NGX_LOG_ERR                                            ,
                       r->connection->log                                     ,
                       0                                                      ,
                       "[Alpaca filter]: could not process html content. If "
                       "you use proxy_pass, set proxy_set_header "
                       "Accept-Encoding \"\" so that the upstream server "
                       "returns raw html, "
                     );

        *response = init_response;
    }
}

// -----------------------------------------------------------------------------------------------------

static ngx_int_t ngx_http_alpaca_header_filter(ngx_http_request_t* r) {
    // setenv("RUST_BACKTRACE", "1", 1);        // for rust debugging

    ngx_http_alpaca_loc_conf_t *plcf;
    ngx_http_alpaca_ctx_t      *ctx;

    plcf = ngx_http_get_module_loc_conf(r, ngx_http_alpaca_module);

    // Call the next filter if neither of the ALPaCA versions have been
    // activated

    // But always serve the fake image, even if the configuration does not
    // enable ALPaCA for the /__alpaca_fake_image.png url
    if ( !is_fake_image(r) && !plcf->prob_enabled && !plcf->deter_enabled )
        return ngx_http_next_header_filter(r);


    // Get the module context
    ctx = ngx_http_get_module_ctx(r, ngx_http_alpaca_module);

    if (ctx == NULL) {

        ctx = ngx_pcalloc( r->pool, sizeof(ngx_http_alpaca_ctx_t) );

        if (ctx == NULL) {
            ngx_log_error(NGX_LOG_ERR, r->connection->log, 0, "[Alpaca filter]: cannot allocate ctx memory");
            return ngx_http_next_header_filter(r);
        }

        ngx_http_set_ctx(r, ctx, ngx_http_alpaca_module);

        // Allocate some space for the whole response if we have an html request
        if ( is_html(r) && !is_fake_image(r) ) {

            ctx->capacity = ( r->headers_out.content_length_n <= 0 ) ? 1000 : r->headers_out.content_length_n;
            ctx->size     = 0;
            ctx->response = ngx_pcalloc(r->pool, ctx->capacity + 1);
            ctx->end      = ctx->response;
        }
    }

    // If the fake alpaca image is requested, change the 404 status to 200
    if (is_fake_image(r) && r->args.len != 0) {
        r->headers_out.status            = 200;
        r->headers_out.content_type.data = (u_char*)"image/png";
        r->headers_out.content_type.len  = 9;
        r->headers_out.content_type_len  = 9;
    }

    // Force reading file buffers into memory buffers
    r->filter_need_in_memory = 1;

    // Reset content length
    ngx_http_clear_content_length(r);

    // Disable ranges
    ngx_http_clear_accept_ranges(r);

    // Clear etag
    ngx_http_clear_etag(r);

    return ngx_http_next_header_filter(r);
}

static ngx_int_t ngx_http_alpaca_body_filter(ngx_http_request_t* r, ngx_chain_t* in) {

    ngx_chain_t  out;

    ngx_http_alpaca_loc_conf_t *plcf;
    ngx_http_core_loc_conf_t   *core_plcf;
    ngx_http_alpaca_ctx_t      *ctx;
    ngx_chain_t                *cl;

    u_char *response; // Response to be sent from the server

    static               map req_mapper = NULL;
    static struct MorphInfo *main_info  = NULL;

    static int subreq_count = 0;
    static int subreq_tbd   = 0;

    // Call the next filter if neither of the ALPaCA versions have been
    // activated But always serve the fake image, even if the configuration does
    // not enable ALPaCA for the /__alpaca_fake_image.png url

    plcf      = ngx_http_get_module_loc_conf(r, ngx_http_alpaca_module);
    core_plcf = ngx_http_get_module_loc_conf(r, ngx_http_core_module);

    if ( !is_fake_image(r) && !plcf->prob_enabled && !plcf->deter_enabled )
        return ngx_http_next_body_filter(r, in);

    // Get the module context
    ctx = ngx_http_get_module_ctx(r, ngx_http_alpaca_module);

    if (ctx == NULL) {
        ngx_log_error( NGX_LOG_ERR, r->connection->log, 0, "[Alpaca filter]: ngx_http_alpaca_module unable to get module ctx" );
        return ngx_http_next_body_filter(r, in);
    }

    // If the fake alpaca image is requested, change some metadata and pad it
    if ( is_fake_image(r) ) {

        // Proceed only if there is an ALPaCA GET parameter
        if (r->args.len == 0)
            return ngx_http_next_body_filter(r, in);

        r->headers_out.status            = 200;
        r->headers_out.content_type.data = (u_char *)"image/png";
        r->headers_out.content_type_len  = 9;

        struct MorphInfo info = {
            .content_type = (u_char*)"image/png"          ,
            .query        = copy_ngx_str(r->args, r->pool),
            .size         = 0                             ,
        };

        // Call ALPaCA to get the padding
        if ( !morph_object(&info) ) {
            // Call the next filter if something went wrong
            return ngx_http_next_body_filter(r, in);
        }

        // Copy the fake object and free the memory that was allocated in rust
        // using the custom "free memory" funtion.
        response = ngx_pcalloc( r->pool, info.size * sizeof(u_char) );

        ngx_memcpy(response, info.content, info.size);
        free_memory(info.content, info.size);
        send_response(r, info.size, response, &out, true);

        return ngx_http_next_body_filter(r, &out);
    }

    // ------------------------------------------------------------------------------------------------------------

    // If the response is an html, wait until the whole body has been
    // captured and morph it according to ALPaCA
    if ( is_html(r) && r->headers_out.status != 404 && r == r->main ) {

        // Iterate through every buffer of the current
        // chain and find its content size
        if ( ( response = get_response(ctx, r, in, true) ) != NULL ) {

            // Re-Initialize subrequests count for page refreshes
            subreq_count = 0;

            // If there are subrequests to be done, don't return the html body
            // because the objects should be padded first
            int8_t subreq_res = execute_subrequests( &main_info, &req_mapper, &subreq_tbd, in, ctx, plcf, core_plcf, r );

            if ( subreq_res == -1 )
                return NGX_ERROR;

            if ( subreq_res ) {
                ngx_http_set_ctx(r, NULL, ngx_http_alpaca_module);
                send_response(r, ctx->size, response, &out, false);

            // If there are no files inside the given html,
            // pad the html body and return it
            } else {
                simple_html_morph(main_info, req_mapper, &response, ctx, r);
                send_response(r, ctx->size, response, &out, true);
            }

            return ngx_http_next_body_filter(r, &out);
        }

        // Do not call the next filter unless the whole html has been captured
        return NGX_OK;

    // It executes when we receive a request
    // for a file, that is not subrequest
    } else if (is_paddable(r) && r == r->main) {

		// Proceed only if there is an ALPaCA GET parameter
		if (r->args.len == 0)
			return ngx_http_next_body_filter(r, in);

		if (get_response(ctx, r, in, true) != NULL) {

            for (cl = in; cl; cl = cl->next) {
				if (cl->buf->last_buf) {
					break;
                }
			}

            ngx_uint_t response_size;

            if ( !pad_object( &response, &response_size, ctx, r) ) {
                // Call the next filter if something went wrong
				return ngx_http_next_body_filter(r, in);
            }

            send_response(r, response_size, response, &out, true);

			cl->buf->last_buf = 0;
			cl->next          = &out;
		}

		return ngx_http_next_body_filter(r, in);

    // Process subrequests
    } else if (r != r->main) {

        if ( ( response = get_response(ctx , r , in , false) ) != NULL ) {

            // Counts how many subrequests have been processed
            subreq_count++;

            // Create and pass request data structure to the map
            map_insert_response(req_mapper, response, ctx, r);

            // ---------------------------------------------------------------

            if ( r->headers_out.status != 404 ) {

                // When True, all subrequests have been processed
                // and we are processing the very last one
				if (subreq_count == subreq_tbd) {

                    // We are processing the last CSS subrequest
                    if ( is_css(r) && plcf->css_inlining_enabled ) {

                        subreq_count = 0;

                        inline_css_content(main_info, req_mapper);

                        execute_html_object_subrequests(main_info, &subreq_tbd, r);

                    // We are processing the last subrequest for HTML objects
                    } else {

                        process_html_objects(req_mapper, main_info, &response, ctx, r);

                        send_response(r, main_info->size, response, &out, true);

                        return ngx_http_next_body_filter(r, &out);
                    }

				}
            }
        }
    }

    return ngx_http_next_body_filter(r, in);
}