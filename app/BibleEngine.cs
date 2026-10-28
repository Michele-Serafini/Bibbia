using System.Runtime.InteropServices;
using System.Text.Json;

namespace Bibbia.Windows;

internal static class BibleEngine
{
    // Rust owns the response. Never pass it to Marshal.FreeHGlobal.
    [DllImport("bible_core", CallingConvention = CallingConvention.Cdecl)]
    private static extern IntPtr bible_request([MarshalAs(UnmanagedType.LPUTF8Str)] string request);
    [DllImport("bible_core", CallingConvention = CallingConvention.Cdecl)]
    private static extern void bible_free(IntPtr response);

    public static unsafe JsonElement Call(object request)
    {
        var pointer = bible_request(JsonSerializer.Serialize(request));
        if (pointer == IntPtr.Zero) throw new InvalidOperationException("Il motore non ha risposto.");
        try
        {
            // Parse the native UTF-8 response directly. Deserialize owns the
            // resulting element; no UTF-16 round trip or second document clone.
            var bytes=MemoryMarshal.CreateReadOnlySpanFromNullTerminated((byte*)pointer);
            var root = JsonSerializer.Deserialize<JsonElement>(bytes);
            if (!root.GetProperty("ok").GetBoolean())
                throw new InvalidOperationException(root.GetProperty("error").GetString());
            return root.GetProperty("data");
        }
        finally { bible_free(pointer); }
    }
}
