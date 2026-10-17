use super::*;
use crate::api::graphics::opengl;
exports! {
export!("gdi32.dll","ChoosePixelFormat","graphics/opengl","partial","native X11 GLX context and pixel format",opengl::choose_pixel_format),
export!("gdi32.dll","DescribePixelFormat","graphics/opengl","partial","native X11 GLX context and pixel format",opengl::describe_pixel_format),
export!("gdi32.dll","SetPixelFormat","graphics/opengl","partial","native X11 GLX context and pixel format",opengl::set_pixel_format),
export!("gdi32.dll","GetPixelFormat","graphics/opengl","partial","native X11 GLX context and pixel format",opengl::get_pixel_format),
export!("gdi32.dll","SwapBuffers","graphics/opengl","partial","native X11 GLX context and pixel format",opengl::swap_buffers),
export!("opengl32.dll","wglCreateContext","graphics/opengl","partial","native X11 GLX context and pixel format",opengl::create_context),
export!("opengl32.dll","wglDeleteContext","graphics/opengl","partial","native X11 GLX context and pixel format",opengl::delete_context),
export!("opengl32.dll","wglMakeCurrent","graphics/opengl","partial","native X11 GLX context and pixel format",opengl::make_current),
export!("opengl32.dll","wglGetCurrentContext","graphics/opengl","partial","native X11 GLX context and pixel format",opengl::get_current_context),
export!("opengl32.dll","wglGetCurrentDC","graphics/opengl","partial","native X11 GLX context and pixel format",opengl::get_current_dc),
export!("opengl32.dll","wglGetProcAddress","graphics/opengl","partial","native X11 GLX context and pixel format",opengl::get_proc_address),
export!("opengl32.dll","wglShareLists","graphics/opengl","partial","native X11 GLX context and pixel format",opengl::share_lists),
}
