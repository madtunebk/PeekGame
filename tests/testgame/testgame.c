/* Windows x64 test "game" for the peek native runner. It does what a D3D11 game does once it
 * starts: registers a window class, creates a window, creates a D3D11 device + DXGI swap chain,
 * pumps messages with PeekMessage(PM_REMOVE), clears and presents frames, handles resize and keys.
 *
 * No C runtime (-nostdlib): every import is a real Win32/D3D API, so the import table is exactly
 * the list peek has to provide. Output goes to stdout (WriteFile) and OutputDebugStringA.
 *
 *   testgame.exe [--frames N]   exit after N presented frames (default 300, ~5 s; 0 = until closed)
 *   peek doesn't forward arguments yet, so runs under peek use the default.
 * Exit code: 0 ok, 1 setup failure, 2 readback mismatch. */
#define COBJMACROS
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <initguid.h>
#include <d3d11.h>
#include <dxgi.h>

/* The compiler may emit calls to these for struct copies/zeroing. */
__attribute__((optimize("no-tree-loop-distribute-patterns")))
void *memset(void *dst, int c, size_t n) { unsigned char *p = dst; while (n--) *p++ = (unsigned char)c; return dst; }
__attribute__((optimize("no-tree-loop-distribute-patterns")))
void *memcpy(void *dst, const void *src, size_t n) {
    unsigned char *d = dst; const unsigned char *s = src; while (n--) *d++ = *s++; return dst;
}

static HANDLE out;

static void put(const char *s) {
    DWORD len = 0, written;
    while (s[len]) len++;
    if (out && out != INVALID_HANDLE_VALUE) WriteFile(out, s, len, &written, NULL);
    OutputDebugStringA(s);
}

static void put_num(const char *label, unsigned long long v, int hex) {
    char buf[64], digits[32];
    int n = 0, i = 0;
    do { unsigned d = (unsigned)(v % (hex ? 16 : 10)); digits[n++] = (char)(d < 10 ? '0' + d : 'a' + d - 10);
         v /= hex ? 16 : 10; } while (v);
    while (*label) buf[i++] = *label++;
    if (hex) { buf[i++] = '0'; buf[i++] = 'x'; }
    while (n) buf[i++] = digits[--n];
    buf[i++] = '\n'; buf[i] = 0;
    put(buf);
}

static int fail(const char *what, HRESULT hr) {
    put("FAIL "); put(what); put_num(" hr=", (unsigned long)hr, 1);
    return 1;
}

static int width = 640, height = 480, resized;

static LRESULT CALLBACK wndproc(HWND hwnd, UINT msg, WPARAM wp, LPARAM lp) {
    switch (msg) {
    case WM_SIZE:
        if (LOWORD(lp) && HIWORD(lp) && (LOWORD(lp) != width || HIWORD(lp) != height)) {
            width = LOWORD(lp); height = HIWORD(lp); resized = 1;
        }
        put_num("WM_SIZE wParam=", wp, 0);
        return 0;
    case WM_KEYDOWN:
        put_num("WM_KEYDOWN vk=", wp, 1);
        if (wp == VK_ESCAPE) PostMessageW(hwnd, WM_CLOSE, 0, 0);
        return 0;
    case WM_KEYUP:
        put_num("WM_KEYUP vk=", wp, 1);
        return 0;
    case WM_CLOSE:
        put("WM_CLOSE\n");
        DestroyWindow(hwnd);
        return 0;
    case WM_DESTROY:
        put("WM_DESTROY\n");
        PostQuitMessage(0);
        return 0;
    }
    return DefWindowProcW(hwnd, msg, wp, lp);
}

static unsigned parse_frames(void) {
    const char *cmd = GetCommandLineA(), *p = cmd;
    static const char flag[] = "--frames";
    for (; *p; p++) {
        int i = 0;
        while (flag[i] && p[i] == flag[i]) i++;
        if (flag[i]) continue;
        p += i;
        while (*p == ' ') p++;
        unsigned v = 0;
        while (*p >= '0' && *p <= '9') v = v * 10 + (unsigned)(*p++ - '0');
        return v;
    }
    return 300;
}

static const float palette[3][4] = { { 0.8f, 0.1f, 0.1f, 1.0f }, { 0.1f, 0.7f, 0.2f, 1.0f }, { 0.1f, 0.3f, 0.9f, 1.0f } };

/* Copies the back buffer to a staging texture and checks the centre pixel against `c`. */
static int check_pixel(ID3D11Device *dev, ID3D11DeviceContext *ctx, ID3D11Texture2D *tex, const float *c) {
    D3D11_TEXTURE2D_DESC desc;
    ID3D11Texture2D_GetDesc(tex, &desc);
    desc.Usage = D3D11_USAGE_STAGING; desc.BindFlags = 0;
    desc.CPUAccessFlags = D3D11_CPU_ACCESS_READ; desc.MiscFlags = 0;
    ID3D11Texture2D *staging = NULL;
    HRESULT hr = ID3D11Device_CreateTexture2D(dev, &desc, NULL, &staging);
    if (FAILED(hr)) return fail("CreateTexture2D(staging)", hr);
    ID3D11DeviceContext_CopyResource(ctx, (ID3D11Resource *)staging, (ID3D11Resource *)tex);
    D3D11_MAPPED_SUBRESOURCE map;
    hr = ID3D11DeviceContext_Map(ctx, (ID3D11Resource *)staging, 0, D3D11_MAP_READ, 0, &map);
    if (FAILED(hr)) { ID3D11Texture2D_Release(staging); return fail("Map", hr); }
    const unsigned char *px = (const unsigned char *)map.pData + (desc.Height / 2) * map.RowPitch + (desc.Width / 2) * 4;
    int ok = 1;
    for (int i = 0; i < 4; i++) {
        int want = (int)(c[i] * 255.0f + 0.5f), d = (int)px[i] - want;
        if (d < -1 || d > 1) ok = 0;
    }
    put_num("readback rgba=", ((unsigned)px[0] << 24) | ((unsigned)px[1] << 16) | ((unsigned)px[2] << 8) | px[3], 1);
    ID3D11DeviceContext_Unmap(ctx, (ID3D11Resource *)staging, 0);
    ID3D11Texture2D_Release(staging);
    if (!ok) { put("FAIL readback mismatch\n"); return 2; }
    return 0;
}

static int run(void) {
    out = GetStdHandle(STD_OUTPUT_HANDLE);
    unsigned max_frames = parse_frames();
    HINSTANCE inst = GetModuleHandleW(NULL);

    WNDCLASSEXW wc = { 0 };
    wc.cbSize = sizeof wc;
    wc.lpfnWndProc = wndproc;
    wc.hInstance = inst;
    wc.hCursor = LoadCursorW(NULL, (LPCWSTR)IDC_ARROW);
    wc.lpszClassName = L"PeekTestGame";
    if (!RegisterClassExW(&wc)) return fail("RegisterClassExW", HRESULT_FROM_WIN32(GetLastError()));

    RECT rc = { 0, 0, width, height };
    AdjustWindowRectEx(&rc, WS_OVERLAPPEDWINDOW, FALSE, 0);
    HWND hwnd = CreateWindowExW(0, wc.lpszClassName, L"peek test game", WS_OVERLAPPEDWINDOW, CW_USEDEFAULT,
                                CW_USEDEFAULT, rc.right - rc.left, rc.bottom - rc.top, NULL, NULL, inst, NULL);
    if (!hwnd) return fail("CreateWindowExW", HRESULT_FROM_WIN32(GetLastError()));
    ShowWindow(hwnd, SW_SHOW);
    put("window created\n");

    DXGI_SWAP_CHAIN_DESC sc = { 0 };
    sc.BufferDesc.Width = (UINT)width; sc.BufferDesc.Height = (UINT)height;
    sc.BufferDesc.Format = DXGI_FORMAT_R8G8B8A8_UNORM;
    sc.SampleDesc.Count = 1;
    sc.BufferUsage = DXGI_USAGE_RENDER_TARGET_OUTPUT;
    sc.BufferCount = 2;
    sc.OutputWindow = hwnd;
    sc.Windowed = TRUE;
    sc.SwapEffect = DXGI_SWAP_EFFECT_FLIP_DISCARD;
    D3D_FEATURE_LEVEL want = D3D_FEATURE_LEVEL_11_0, got;
    IDXGISwapChain *swap = NULL; ID3D11Device *dev = NULL; ID3D11DeviceContext *ctx = NULL;
    HRESULT hr = D3D11CreateDeviceAndSwapChain(NULL, D3D_DRIVER_TYPE_HARDWARE, NULL, 0, &want, 1, D3D11_SDK_VERSION,
                                               &sc, &swap, &dev, &got, &ctx);
    if (FAILED(hr)) return fail("D3D11CreateDeviceAndSwapChain", hr);
    put_num("D3D11 device created, feature level ", got, 1);

    int result = 0;
    unsigned frame = 0;
    MSG msg;
    for (;;) {
        int quit = 0;
        while (PeekMessageW(&msg, NULL, 0, 0, PM_REMOVE)) {
            if (msg.message == WM_QUIT) { quit = 1; break; }
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        if (quit) break;
        if (resized) {
            ID3D11DeviceContext_OMSetRenderTargets(ctx, 0, NULL, NULL);
            hr = IDXGISwapChain_ResizeBuffers(swap, 0, (UINT)width, (UINT)height, DXGI_FORMAT_UNKNOWN, 0);
            if (FAILED(hr)) { result = fail("ResizeBuffers", hr); break; }
            put_num("resized width=", (unsigned)width, 0);
            resized = 0;
        }
        ID3D11Texture2D *back = NULL; ID3D11RenderTargetView *rtv = NULL;
        hr = IDXGISwapChain_GetBuffer(swap, 0, &IID_ID3D11Texture2D, (void **)&back);
        if (FAILED(hr)) { result = fail("GetBuffer", hr); break; }
        hr = ID3D11Device_CreateRenderTargetView(dev, (ID3D11Resource *)back, NULL, &rtv);
        if (FAILED(hr)) { ID3D11Texture2D_Release(back); result = fail("CreateRenderTargetView", hr); break; }
        const float *color = palette[(frame / 60) % 3];
        ID3D11DeviceContext_OMSetRenderTargets(ctx, 1, &rtv, NULL);
        ID3D11DeviceContext_ClearRenderTargetView(ctx, rtv, color);
        if (frame == 0) result = check_pixel(dev, ctx, back, color);
        ID3D11RenderTargetView_Release(rtv);
        ID3D11Texture2D_Release(back);
        if (result) break;
        hr = IDXGISwapChain_Present(swap, 1, 0);
        if (FAILED(hr)) { result = fail("Present", hr); break; }
        frame++;
        if (max_frames && frame >= max_frames) { put("frame limit reached\n"); DestroyWindow(hwnd); }
    }
    put_num("frames presented: ", frame, 0);
    ID3D11DeviceContext_ClearState(ctx);
    ID3D11DeviceContext_Release(ctx);
    ID3D11Device_Release(dev);
    IDXGISwapChain_Release(swap);
    put(result ? "RESULT FAIL\n" : "RESULT PASS\n");
    return result;
}

void entry(void) { ExitProcess((UINT)run()); }
