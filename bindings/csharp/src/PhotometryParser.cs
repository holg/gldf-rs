using uniffi.gldf_ffi;

namespace Gldf.Net;

/// <summary>
/// Static utilities for parsing EULUMDAT (.ldt) and IES photometric data files.
/// </summary>
public static class PhotometryParser
{
    /// <summary>Parse EULUMDAT content from a string.</summary>
    /// <param name="content">EULUMDAT file content as string.</param>
    /// <returns>Parsed photometric data including intensity distribution.</returns>
    public static EulumdatData ParseEulumdat(string content)
        => GldfFfiMethods.ParseEulumdat(content);

    /// <summary>Parse EULUMDAT content from a byte array.</summary>
    /// <param name="data">EULUMDAT file content as bytes.</param>
    /// <returns>Parsed photometric data including intensity distribution.</returns>
    public static EulumdatData ParseEulumdatBytes(byte[] data)
        => GldfFfiMethods.ParseEulumdatBytes(data);
}
