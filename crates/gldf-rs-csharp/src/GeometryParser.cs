using uniffi.gldf_ffi;

namespace Gldf.Net;

/// <summary>
/// Static utilities for parsing L3D 3D geometry files.
/// </summary>
public static class GeometryParser
{
    /// <summary>Parse an L3D file from a byte array (ZIP archive).</summary>
    /// <param name="data">L3D file content as bytes.</param>
    /// <returns>Parsed L3D file with scene structure and embedded assets.</returns>
    public static L3dFile ParseL3d(byte[] data)
        => GldfFfiMethods.ParseL3d(data);

    /// <summary>Parse L3D scene structure from XML content.</summary>
    /// <param name="xmlContent">L3D XML structure content as string.</param>
    /// <returns>Parsed L3D scene with parts, joints, and geometry definitions.</returns>
    public static L3dScene ParseL3dStructure(string xmlContent)
        => GldfFfiMethods.ParseL3dStructure(xmlContent);

    /// <summary>Get an asset from a parsed L3D file by filename.</summary>
    /// <param name="l3dFile">Parsed L3D file.</param>
    /// <param name="filename">Asset filename to retrieve.</param>
    /// <returns>Asset data as byte array, or null if not found.</returns>
    public static byte[]? GetL3dAsset(L3dFile l3dFile, string filename)
        => GldfFfiMethods.GetL3dAsset(l3dFile, filename);
}
