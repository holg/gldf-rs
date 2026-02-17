using uniffi.gldf_ffi;

namespace Gldf.Net;

/// <summary>
/// High-level API for working with GLDF (Global Lighting Data Format) files.
/// Wraps the Rust-powered gldf-rs engine via UniFFI bindings.
/// </summary>
public sealed class GldfDocument : IDisposable
{
    private readonly GldfEngine _engine;
    private bool _disposed;

    private GldfDocument(GldfEngine engine) => _engine = engine;

    /// <summary>Load a GLDF file from a byte array (ZIP archive).</summary>
    public static GldfDocument FromBytes(byte[] data)
        => new(GldfEngine.FromBytes(data));

    /// <summary>Load a GLDF file from a JSON string.</summary>
    public static GldfDocument FromJson(string json)
        => new(GldfEngine.FromJson(json));

    /// <summary>Create a new empty GLDF document.</summary>
    public static GldfDocument CreateEmpty()
        => new(GldfEngine.NewEmpty());

    // --- Read operations ---

    /// <summary>Get header information (manufacturer, author, format version, etc.).</summary>
    public GldfHeader Header => _engine.GetHeader();

    /// <summary>Get document statistics (file counts, variant counts, etc.).</summary>
    public GldfStats Stats => _engine.GetStats();

    /// <summary>Get all file definitions.</summary>
    public List<GldfFile> Files => _engine.GetFiles();

    /// <summary>Get photometric file definitions (.ldt, .ies).</summary>
    public List<GldfFile> PhotometricFiles => _engine.GetPhotometricFiles();

    /// <summary>Get image file definitions.</summary>
    public List<GldfFile> ImageFiles => _engine.GetImageFiles();

    /// <summary>Get geometry file definitions (.l3d).</summary>
    public List<GldfFile> GeometryFiles => _engine.GetGeometryFiles();

    /// <summary>Get all product variants.</summary>
    public List<GldfVariant> Variants => _engine.GetVariants();

    /// <summary>Get all light sources (fixed and changeable).</summary>
    public List<GldfLightSource> LightSources => _engine.GetLightSources();

    /// <summary>Get electrical attributes.</summary>
    public GldfElectrical Electrical => _engine.GetElectrical();

    /// <summary>Get application categories.</summary>
    public List<string> Applications => _engine.GetApplications();

    /// <summary>Whether the document has been modified since last save.</summary>
    public bool IsModified => _engine.IsModified();

    /// <summary>Whether the document has archive data (loaded from .gldf ZIP).</summary>
    public bool HasArchiveData => _engine.HasArchiveData();

    // --- Write operations ---

    /// <summary>Set the manufacturer name.</summary>
    public void SetManufacturer(string value) => _engine.SetManufacturer(value);

    /// <summary>Set the author name.</summary>
    public void SetAuthor(string value) => _engine.SetAuthor(value);

    /// <summary>Set the creation time code.</summary>
    public void SetCreationTimeCode(string value) => _engine.SetCreationTimeCode(value);

    /// <summary>Set the application that created this file.</summary>
    public void SetCreatedWithApplication(string value) => _engine.SetCreatedWithApplication(value);

    /// <summary>Set the default language.</summary>
    public void SetDefaultLanguage(string? value) => _engine.SetDefaultLanguage(value);

    /// <summary>Set the format version.</summary>
    public void SetFormatVersion(string value) => _engine.SetFormatVersion(value);

    /// <summary>Set electrical safety class.</summary>
    public void SetElectricalSafetyClass(string? value) => _engine.SetElectricalSafetyClass(value);

    /// <summary>Set IP code.</summary>
    public void SetIpCode(string? value) => _engine.SetIpCode(value);

    /// <summary>Set power factor.</summary>
    public void SetPowerFactor(double? value) => _engine.SetPowerFactor(value);

    /// <summary>Set constant light output flag.</summary>
    public void SetConstantLightOutput(bool? value) => _engine.SetConstantLightOutput(value);

    /// <summary>Set light distribution type.</summary>
    public void SetLightDistribution(string? value) => _engine.SetLightDistribution(value);

    /// <summary>Set switching capacity.</summary>
    public void SetSwitchingCapacity(string? value) => _engine.SetSwitchingCapacity(value);

    // --- File operations ---

    /// <summary>Add a file definition.</summary>
    public void AddFile(string id, string fileName, string contentType, string fileType)
        => _engine.AddFile(id, fileName, contentType, fileType);

    /// <summary>Remove a file definition by ID.</summary>
    public void RemoveFile(string id) => _engine.RemoveFile(id);

    /// <summary>Update a file definition.</summary>
    public void UpdateFile(string id, string? fileName, string? contentType, string? fileType)
        => _engine.UpdateFile(id, fileName ?? "", contentType ?? "", fileType ?? "");

    /// <summary>Get file content by file ID.</summary>
    public GldfFileContent GetFileContent(string fileId) => _engine.GetFileContent(fileId);

    /// <summary>Get file content as string by file ID.</summary>
    public string GetFileContentAsString(string fileId) => _engine.GetFileContentAsString(fileId);

    /// <summary>Get geometry content by geometry ID.</summary>
    public GldfFileContent GetGeometryContent(string geometryId) => _engine.GetGeometryContent(geometryId);

    /// <summary>Get emitter data for a given emitter ID.</summary>
    public GldfEmitterData? GetEmitterData(string emitterId) => _engine.GetEmitterData(emitterId);

    /// <summary>List all files in the GLDF archive.</summary>
    public List<string> ListArchiveFiles() => _engine.ListArchiveFiles();

    /// <summary>Get a file from the archive by path.</summary>
    public byte[] GetArchiveFile(string path) => _engine.GetArchiveFile(path);

    // --- Application operations ---

    /// <summary>Add an application category.</summary>
    public void AddApplication(string application) => _engine.AddApplication(application);

    /// <summary>Remove an application category by index.</summary>
    public void RemoveApplication(uint index) => _engine.RemoveApplication(index);

    /// <summary>Set all application categories.</summary>
    public void SetApplications(List<string> applications) => _engine.SetApplications(applications);

    // --- Export ---

    /// <summary>Export to JSON string.</summary>
    public string ToJson() => _engine.ToJson();

    /// <summary>Export to pretty-printed JSON string.</summary>
    public string ToPrettyJson() => _engine.ToPrettyJson();

    /// <summary>Export to XML string.</summary>
    public string ToXml() => _engine.ToXml();

    /// <summary>Mark the document as saved (clears modified flag).</summary>
    public void MarkSaved() => _engine.MarkSaved();

    // --- IFC/BIM Integration ---

    /// <summary>Export this GLDF document to IFC STEP format for BIM integration.</summary>
    public string ExportToIfc() => _engine.ExportToIfc();

    /// <summary>Import luminaire data from an IFC file content string.</summary>
    public IfcImportedLuminaire ImportFromIfc(string ifcContent) => _engine.ImportFromIfc(ifcContent);

    public void Dispose()
    {
        if (!_disposed)
        {
            _engine.Dispose();
            _disposed = true;
        }
    }
}
