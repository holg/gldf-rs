using uniffi.gldf_ffi;

namespace Gldf.Net;

/// <summary>
/// Static utilities for IFC/GLDF conversion without loading a full GldfDocument.
/// </summary>
public static class IfcConverter
{
    /// <summary>Parse IFC STEP content and extract luminaire data.</summary>
    /// <param name="ifcContent">IFC STEP file content as string.</param>
    /// <returns>Imported luminaire with variants, light sources, and properties.</returns>
    public static IfcImportedLuminaire Import(string ifcContent)
        => GldfFfiMethods.IfcImport(ifcContent);

    /// <summary>Convert IFC STEP content directly to GLDF bytes (ZIP archive).</summary>
    /// <param name="ifcContent">IFC STEP file content as string.</param>
    /// <returns>GLDF file as byte array (ZIP archive).</returns>
    public static byte[] ToGldfBytes(string ifcContent)
        => GldfFfiMethods.IfcToGldfBytes(ifcContent);
}
