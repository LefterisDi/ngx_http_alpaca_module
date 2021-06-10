import os
import pathlib
import chromedriver_autoinstaller

from utils            import colors
from response_handler import get_response_filenames

methods = {
    'deter_simple_less_objs'      : 'Deterministic/nginx_simple_less_objs.conf'     ,
    'deter_simple_more_objs'      : 'Deterministic/nginx_simple_more_objs.conf'     ,
    'deter_simple_exact_objs'     : 'Deterministic/nginx_simple_exact_objs.conf'    ,
    'deter_relocation'            : 'Deterministic/nginx_relocation.conf'           ,
    'deter_inline_some'           : 'Deterministic/nginx_inline_some.conf'          ,
    'deter_inline_some_incl_css'  : 'Deterministic/nginx_inline_some_incl_css.conf' ,
    'deter_inline_some_force_css' : 'Deterministic/nginx_inline_some_force_css.conf',
    'deter_inline_all'            : 'Deterministic/nginx_inline_all.conf'           ,
    'deter_inline_all_incl_css'   : 'Deterministic/nginx_inline_all_incl_css.conf'  ,
    'deter_inline_all_force_css'  : 'Deterministic/nginx_inline_all_force_css.conf' ,
    'deter_inline_more'           : 'Deterministic/nginx_inline_more.conf'          ,
    'deter_force_css'             : 'Deterministic/nginx_force_css.conf'            ,

    'prob_simple_less_objs'       : 'Probabilistic/nginx_simple_less_objs.conf'     ,
    'prob_simple_more_objs'       : 'Probabilistic/nginx_simple_more_objs.conf'     ,
    'prob_simple_exact_objs'      : 'Probabilistic/nginx_simple_exact_objs.conf'    ,
    'prob_relocation'             : 'Probabilistic/nginx_relocation.conf'           ,
    'prob_inline_some'            : 'Probabilistic/nginx_inline_some.conf'          ,
    'prob_inline_some_incl_css'   : 'Probabilistic/nginx_inline_some_incl_css.conf' ,
    'prob_inline_some_force_css'  : 'Probabilistic/nginx_inline_some_force_css.conf',
    'prob_inline_all'             : 'Probabilistic/nginx_inline_all.conf'           ,
    'prob_inline_all_incl_css'    : 'Probabilistic/nginx_inline_all_incl_css.conf'  ,
    'prob_inline_all_force_css'   : 'Probabilistic/nginx_inline_all_force_css.conf' ,
    'prob_inline_more'            : 'Probabilistic/nginx_inline_more.conf'          ,
    'prob_force_css'              : 'Probabilistic/nginx_force_css.conf'            ,
}

inlines = {
    'deter_simple_less_objs'      : 0,
    'deter_simple_more_objs'      : 0,
    'deter_simple_exact_objs'     : 0,
    'deter_relocation'            : 0,
    'deter_inline_some'           : 1,
    'deter_inline_some_incl_css'  : 2,
    'deter_inline_some_force_css' : 3,
    'deter_inline_all'            : 2,
    'deter_inline_all_incl_css'   : 2,
    'deter_inline_all_force_css'  : 3,
    'deter_inline_more'           : 0,
    'deter_force_css'             : 0,

    'prob_simple_less_objs'       : 0,
    'prob_simple_more_objs'       : 0,
    'prob_simple_exact_objs'      : 0,
    'prob_fake_imgs'              : 0,
    'prob_relocation'             : 0,
    'prob_inline_some'            : 1,
    'prob_inline_some_incl_css'   : 2,
    'prob_inline_some_force_css'  : 3,
    'prob_inline_all'             : 2,
    'prob_inline_all_incl_css'    : 2,
    'prob_inline_all_force_css'   : 3,
    'prob_inline_more'            : 0,
    'prob_force_css'              : 0,
}

fake_imgs = {
    'deter_simple_less_objs'      : 1,
    'deter_simple_more_objs'      : 4,
    'deter_simple_exact_objs'     : 0,
    'deter_relocation'            : 2,
    'deter_inline_some'           : 0,
    'deter_inline_some_incl_css'  : 0,
    'deter_inline_some_force_css' : 0,
    'deter_inline_all'            : 0,
    'deter_inline_all_incl_css'   : 0,
    'deter_inline_all_force_css'  : 0,
    'deter_inline_more'           : 4,
    'deter_force_css'             : 2,

    'prob_simple_less_objs'       : 0,
    'prob_simple_more_objs'       : 4,
    'prob_simple_exact_objs'      : 0,
    'prob_fake_imgs'              : [1,2,3,4],
    'prob_relocation'             : 2,
    'prob_inline_some'            : 0,
    'prob_inline_some_incl_css'   : 0,
    'prob_inline_some_force_css'  : 0,
    'prob_inline_all'             : 0,
    'prob_inline_all_incl_css'    : 0,
    'prob_inline_all_force_css'   : 0,
    'prob_inline_more'            : 4,
    'prob_force_css'              : 0,
}

returned_css = {
    'deter_simple_less_objs'      : 1,
    'deter_simple_more_objs'      : 1,
    'deter_simple_exact_objs'     : 1,
    'deter_relocation'            : 1,
    'deter_inline_some'           : [0,1],
    'deter_inline_some_incl_css'  : [0,1],
    'deter_inline_some_force_css' : [0,1],
    'deter_inline_all'            : 1,
    'deter_inline_all_incl_css'   : 0,
    'deter_inline_all_force_css'  : 0,
    'deter_inline_more'           : 1,
    'deter_force_css'             : 0,

    'prob_simple_less_objs'       : 1,
    'prob_simple_more_objs'       : 1,
    'prob_simple_exact_objs'      : 1,
    'prob_fake_imgs'              : 1,
    'prob_relocation'             : 1,
    'prob_inline_some'            : [0,1],
    'prob_inline_some_incl_css'   : [0,1],
    'prob_inline_some_force_css'  : [0,1],
    'prob_inline_all'             : 1,
    'prob_inline_all_incl_css'    : 0,
    'prob_inline_all_force_css'   : 0,
    'prob_inline_more'            : 1,
    'prob_force_css'              : 0,
}

success_msg = {
    True  : "finished {}successfully{}!".format(colors.GREENISH, colors.RESET),
    False : "{}failed{}!".format(colors.RED, colors.RESET)
}

"""
    grep removes error below:
    nginx: [alert] could not open error log file: open() "/var/log/nginx/error.log" failed (13: Permission denied)
"""

def run_nginx(conf):
    os.system( "{0}/../build/nginx-1.18.0/objs/nginx -c {0}/{1} 2>&1 | grep -v '/var/log/nginx/error.log'".format( pathlib.Path(__file__).parent.absolute(), conf ) )


def get_alpaca_target_size(file):
    return int( file.split('alpaca-padding=')[1] )


if __name__ == "__main__":

    chromedriver_autoinstaller.install(True)

    chrome_ver        = chromedriver_autoinstaller.get_chrome_version().split('.')[0]
    chromedriver_path = "./{}/chromedriver".format(chrome_ver)

    os.system("fuser -k 8888/tcp >/dev/null 2>&1")

    for conf_name in methods:
        success = True
        run_nginx( methods[conf_name] )

        url = 'http://localhost:8888'
        resp_files , timed_out = get_response_filenames(url, chromedriver_path)

        if (timed_out):
            print("Connection Timed Out!")
            success = False

        else:
            inl_num      = 0
            fake_img_num = 0
            css_num      = 0

            for resp, status, resource_size, transfer_size in resp_files:

                if "data:image" in resp:
                    inl_num += 1

                elif "__alpaca_fake_image.png" in resp:
                    fake_img_num += 1

                elif ".css" in resp:
                    css_num += 1

                try:
                    target_size = get_alpaca_target_size(resp)

                    if resource_size != target_size:
                        print("Error expected sizes defers from real size (expected: {} | real: {})".format(resource_size , target_size) )
                        success = False
                        break
                except:
                    pass

            # print(resp , status , resource_size , transfer_size)
            if success == True and inl_num != inlines[conf_name]:
                print("Inlining error in {}. Expected {} inlined objects and received {}.".format(conf_name, inlines[conf_name], inl_num))
                success = False


            if not isinstance(fake_imgs[conf_name], list):
                if success == True and fake_img_num != fake_imgs[conf_name]:
                    print("Fake images error in {}. Expected {} fake images and received {}.".format(conf_name, fake_imgs[conf_name], fake_img_num))
                    success = False

            elif success == True and fake_img_num not in fake_imgs[conf_name]:
                print("Fake images error in {}. Expected {} fake images and received {}.".format(conf_name, fake_imgs[conf_name], fake_img_num))
                success = False


            if not isinstance(returned_css[conf_name], list):
                if success == True and css_num != returned_css[conf_name]:
                    print("Returned CSS error in {}. Expected {} CSS to be returned and received {}.".format(conf_name, returned_css[conf_name], css_num))
                    success = False

            elif success == True and css_num not in returned_css[conf_name]:
                print("Returned CSS error in {}. Expected {} CSS to be returned and received {}.".format(conf_name, returned_css[conf_name], css_num))
                success = False

        print("{:27} : {}".format(conf_name, success_msg[success]))

        os.system("fuser -k 8888/tcp >/dev/null 2>&1")