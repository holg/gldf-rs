package eu.trahe.gldfviewer

import android.graphics.BitmapFactory
import android.net.Uri
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.compose.setContent
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.Image
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.ArrowBack
import androidx.compose.material.icons.filled.Check
import androidx.compose.material.icons.filled.Close
import androidx.compose.material.icons.filled.Create
import androidx.compose.material.icons.filled.Face
import androidx.compose.material.icons.filled.Home
import androidx.compose.material.icons.filled.Info
import androidx.compose.material.icons.filled.List
import androidx.compose.material.icons.filled.Menu
import androidx.compose.material.icons.filled.Star
import androidx.compose.material.icons.filled.Warning
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.nativeCanvas
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import eu.trahe.gldfviewer.ui.theme.GldfViewerTheme
import uniffi.gldf_ffi.*
import kotlin.math.cos
import kotlin.math.sin
import kotlin.math.PI

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val initialUri = intent?.data

        setContent {
            GldfViewerTheme {
                Surface(
                    modifier = Modifier.fillMaxSize(),
                    color = MaterialTheme.colorScheme.background
                ) {
                    GldfViewerApp(initialUri)
                }
            }
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun GldfViewerApp(initialUri: Uri?) {
    val context = LocalContext.current
    var engine by remember { mutableStateOf<GldfEngine?>(null) }
    var isLoading by remember { mutableStateOf(false) }
    var error by remember { mutableStateOf<String?>(null) }
    var selectedTab by remember { mutableStateOf(0) }

    val filePicker = rememberLauncherForActivityResult(
        contract = ActivityResultContracts.OpenDocument()
    ) { uri ->
        uri?.let {
            isLoading = true
            loadGldfFile(context, it) { result ->
                result.onSuccess { eng ->
                    engine = eng
                    error = null
                }.onFailure { e ->
                    error = e.message
                    engine = null
                }
                isLoading = false
            }
        }
    }

    LaunchedEffect(initialUri) {
        initialUri?.let { uri ->
            isLoading = true
            loadGldfFile(context, uri) { result ->
                result.onSuccess { eng ->
                    engine = eng
                    error = null
                }.onFailure { e ->
                    error = e.message
                    engine = null
                }
                isLoading = false
            }
        }
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("GLDF Viewer") },
                actions = {
                    IconButton(onClick = { filePicker.launch(arrayOf("*/*")) }) {
                        Icon(Icons.Default.Add, contentDescription = "Open File")
                    }
                },
                colors = TopAppBarDefaults.topAppBarColors(
                    containerColor = MaterialTheme.colorScheme.primaryContainer
                )
            )
        },
        bottomBar = {
            if (engine != null) {
                NavigationBar {
                    NavigationBarItem(
                        icon = { Icon(Icons.Default.Info, contentDescription = "Overview") },
                        label = { Text("Overview") },
                        selected = selectedTab == 0,
                        onClick = { selectedTab = 0 }
                    )
                    NavigationBarItem(
                        icon = { Icon(Icons.Default.List, contentDescription = "Files") },
                        label = { Text("Files") },
                        selected = selectedTab == 1,
                        onClick = { selectedTab = 1 }
                    )
                    NavigationBarItem(
                        icon = { Icon(Icons.Default.Star, contentDescription = "Light Sources") },
                        label = { Text("Lights") },
                        selected = selectedTab == 2,
                        onClick = { selectedTab = 2 }
                    )
                    NavigationBarItem(
                        icon = { Icon(Icons.Default.Menu, contentDescription = "Raw") },
                        label = { Text("Raw") },
                        selected = selectedTab == 3,
                        onClick = { selectedTab = 3 }
                    )
                }
            }
        }
    ) { padding ->
        Box(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
        ) {
            when {
                isLoading -> {
                    CircularProgressIndicator(modifier = Modifier.align(Alignment.Center))
                }
                error != null -> {
                    Column(
                        modifier = Modifier.align(Alignment.Center),
                        horizontalAlignment = Alignment.CenterHorizontally
                    ) {
                        Icon(
                            Icons.Default.Warning,
                            contentDescription = "Error",
                            tint = MaterialTheme.colorScheme.error,
                            modifier = Modifier.size(48.dp)
                        )
                        Spacer(modifier = Modifier.height(16.dp))
                        Text(text = error ?: "Unknown error", color = MaterialTheme.colorScheme.error)
                    }
                }
                engine != null -> {
                    when (selectedTab) {
                        0 -> OverviewTab(engine!!)
                        1 -> FilesTab(engine!!)
                        2 -> LightSourcesTab(engine!!)
                        3 -> RawTab(engine!!)
                    }
                }
                else -> {
                    WelcomeScreen(onOpenFile = { filePicker.launch(arrayOf("*/*")) })
                }
            }
        }
    }
}

@Composable
fun WelcomeScreen(onOpenFile: () -> Unit) {
    Column(
        modifier = Modifier.fillMaxSize(),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.Center
    ) {
        Icon(
            Icons.Default.Star,
            contentDescription = "GLDF",
            modifier = Modifier.size(96.dp),
            tint = MaterialTheme.colorScheme.primary
        )
        Spacer(modifier = Modifier.height(24.dp))
        Text(text = "GLDF Viewer", fontSize = 28.sp, fontWeight = FontWeight.Bold)
        Spacer(modifier = Modifier.height(8.dp))
        Text(
            text = "View and explore GLDF lighting data files",
            color = MaterialTheme.colorScheme.onSurfaceVariant
        )
        Spacer(modifier = Modifier.height(32.dp))
        Button(onClick = onOpenFile) {
            Icon(Icons.Default.Add, contentDescription = null)
            Spacer(modifier = Modifier.width(8.dp))
            Text("Open GLDF File")
        }
        Spacer(modifier = Modifier.height(24.dp))
        Text(
            text = "v${gldfLibraryVersion()}",
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            fontSize = 12.sp
        )
    }
}

@Composable
fun OverviewTab(engine: GldfEngine) {
    val header = engine.getHeader()
    val stats = engine.getStats()

    LazyColumn(
        modifier = Modifier.fillMaxSize().padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(16.dp)
    ) {
        item {
            Card(modifier = Modifier.fillMaxWidth()) {
                Column(modifier = Modifier.padding(16.dp)) {
                    Text("Header", fontWeight = FontWeight.Bold, fontSize = 18.sp)
                    Spacer(modifier = Modifier.height(8.dp))
                    InfoRow("Manufacturer", header.manufacturer)
                    InfoRow("Author", header.author)
                    InfoRow("Format Version", header.formatVersion)
                    InfoRow("Created With", header.createdWithApplication)
                    InfoRow("Creation Time", header.creationTimeCode)
                }
            }
        }
        item {
            Card(modifier = Modifier.fillMaxWidth()) {
                Column(modifier = Modifier.padding(16.dp)) {
                    Text("Statistics", fontWeight = FontWeight.Bold, fontSize = 18.sp)
                    Spacer(modifier = Modifier.height(8.dp))
                    InfoRow("Files", stats.filesCount.toString())
                    InfoRow("Fixed Light Sources", stats.fixedLightSourcesCount.toString())
                    InfoRow("Changeable Light Sources", stats.changeableLightSourcesCount.toString())
                    InfoRow("Variants", stats.variantsCount.toString())
                    InfoRow("Photometries", stats.photometriesCount.toString())
                    InfoRow("Simple Geometries", stats.simpleGeometriesCount.toString())
                    InfoRow("Model Geometries", stats.modelGeometriesCount.toString())
                }
            }
        }
    }
}

@Composable
fun InfoRow(label: String, value: String) {
    Row(
        modifier = Modifier.fillMaxWidth().padding(vertical = 4.dp),
        horizontalArrangement = Arrangement.SpaceBetween
    ) {
        Text(label, color = MaterialTheme.colorScheme.onSurfaceVariant)
        Text(value, fontWeight = FontWeight.Medium)
    }
}

@Composable
fun FilesTab(engine: GldfEngine) {
    val files = engine.getFiles()
    var selectedFile by remember { mutableStateOf<GldfFile?>(null) }

    if (selectedFile != null) {
        FileDetailView(
            engine = engine,
            file = selectedFile!!,
            onBack = { selectedFile = null }
        )
    } else {
        LazyColumn(modifier = Modifier.fillMaxSize()) {
            items(files) { file ->
                ListItem(
                    headlineContent = { Text(file.fileName) },
                    supportingContent = { Text(file.contentType) },
                    leadingContent = {
                        Icon(
                            when {
                                file.contentType.startsWith("image") -> Icons.Default.Face
                                file.contentType.startsWith("ldc") -> Icons.Default.Star
                                file.contentType == "geo/l3d" -> Icons.Default.Home
                                else -> Icons.Default.List
                            },
                            contentDescription = null
                        )
                    },
                    trailingContent = {
                        Text(file.fileType, fontSize = 12.sp, color = MaterialTheme.colorScheme.onSurfaceVariant)
                    },
                    modifier = Modifier.clickable { selectedFile = file }
                )
                Divider()
            }
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun FileDetailView(engine: GldfEngine, file: GldfFile, onBack: () -> Unit) {
    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(file.fileName, maxLines = 1) },
                navigationIcon = {
                    IconButton(onClick = onBack) {
                        Icon(Icons.Default.ArrowBack, contentDescription = "Back")
                    }
                }
            )
        }
    ) { padding ->
        Box(modifier = Modifier.padding(padding).fillMaxSize()) {
            when {
                file.contentType.startsWith("image") -> ImageViewer(engine, file)
                file.contentType.startsWith("ldc") -> PhotometryViewer(engine, file)
                file.contentType == "geo/l3d" -> L3dViewer(engine, file)
                else -> GenericFileViewer(engine, file)
            }
        }
    }
}

@Composable
fun ImageViewer(engine: GldfEngine, file: GldfFile) {
    var bitmap by remember { mutableStateOf<android.graphics.Bitmap?>(null) }
    var error by remember { mutableStateOf<String?>(null) }
    var isLoading by remember { mutableStateOf(true) }

    LaunchedEffect(file.id) {
        try {
            val content = engine.getFileContent(file.id)
            val bytes = content.data.map { it.toByte() }.toByteArray()
            bitmap = BitmapFactory.decodeByteArray(bytes, 0, bytes.size)
            if (bitmap == null) {
                error = "Could not decode image"
            }
        } catch (e: Exception) {
            error = e.message
        }
        isLoading = false
    }

    Box(modifier = Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
        when {
            isLoading -> CircularProgressIndicator()
            error != null -> {
                Column(horizontalAlignment = Alignment.CenterHorizontally) {
                    Icon(Icons.Default.Warning, contentDescription = "Error", tint = MaterialTheme.colorScheme.error)
                    Text(error ?: "Unknown error", color = MaterialTheme.colorScheme.error)
                }
            }
            bitmap != null -> {
                Image(
                    bitmap = bitmap!!.asImageBitmap(),
                    contentDescription = file.fileName,
                    modifier = Modifier.fillMaxSize().padding(16.dp)
                )
            }
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun PhotometryViewer(engine: GldfEngine, file: GldfFile) {
    var eulumdatData by remember { mutableStateOf<EulumdatData?>(null) }
    var error by remember { mutableStateOf<String?>(null) }
    var isLoading by remember { mutableStateOf(true) }
    var isPolarView by remember { mutableStateOf(true) }

    LaunchedEffect(file.id) {
        try {
            val content = engine.getFileContent(file.id)
            eulumdatData = parseEulumdatBytes(content.data)
        } catch (e: Exception) {
            error = e.message
        }
        isLoading = false
    }

    when {
        isLoading -> Box(modifier = Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
            CircularProgressIndicator()
        }
        error != null -> Box(modifier = Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
            Column(horizontalAlignment = Alignment.CenterHorizontally) {
                Icon(Icons.Default.Warning, contentDescription = "Error", tint = MaterialTheme.colorScheme.error)
                Text(error ?: "Unknown error", color = MaterialTheme.colorScheme.error)
            }
        }
        eulumdatData != null -> {
            Column(modifier = Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(16.dp)) {
                // Header info
                Card(modifier = Modifier.fillMaxWidth()) {
                    Column(modifier = Modifier.padding(16.dp)) {
                        Text("Photometry Info", fontWeight = FontWeight.Bold, fontSize = 18.sp)
                        Spacer(modifier = Modifier.height(8.dp))
                        InfoRow("Luminaire", eulumdatData!!.luminaireName)
                        InfoRow("Manufacturer", eulumdatData!!.manufacturer)
                        InfoRow("Lamp Type", eulumdatData!!.lampType)
                        InfoRow("Total Lumens", "${eulumdatData!!.totalLumens.toInt()} lm")
                        InfoRow("Wattage", "${eulumdatData!!.wattage.toInt()} W")
                        InfoRow("LORL", "${eulumdatData!!.lorl.toInt()}%")
                        InfoRow("Max Intensity", "${eulumdatData!!.maxIntensity.toInt()} cd/klm")
                    }
                }
                Spacer(modifier = Modifier.height(16.dp))

                // View toggle
                Card(modifier = Modifier.fillMaxWidth()) {
                    Column(modifier = Modifier.padding(16.dp)) {
                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.SpaceBetween,
                            verticalAlignment = Alignment.CenterVertically
                        ) {
                            Text("Light Distribution", fontWeight = FontWeight.Bold, fontSize = 18.sp)
                            Row {
                                FilterChip(
                                    selected = isPolarView,
                                    onClick = { isPolarView = true },
                                    label = { Text("Polar") }
                                )
                                Spacer(modifier = Modifier.width(8.dp))
                                FilterChip(
                                    selected = !isPolarView,
                                    onClick = { isPolarView = false },
                                    label = { Text("Cartesian") }
                                )
                            }
                        }
                        Spacer(modifier = Modifier.height(16.dp))
                        if (isPolarView) {
                            PolarDiagram(
                                eulumdatData = eulumdatData!!,
                                modifier = Modifier.fillMaxWidth().height(350.dp)
                            )
                        } else {
                            CartesianDiagram(
                                eulumdatData = eulumdatData!!,
                                modifier = Modifier.fillMaxWidth().height(300.dp)
                            )
                        }
                    }
                }
            }
        }
    }
}

@Composable
fun PolarDiagram(eulumdatData: EulumdatData, modifier: Modifier = Modifier) {
    val primaryColor = MaterialTheme.colorScheme.primary
    val secondaryColor = MaterialTheme.colorScheme.tertiary
    val onSurfaceColor = MaterialTheme.colorScheme.onSurface

    Canvas(modifier = modifier) {
        val centerX = size.width / 2
        val centerY = size.height / 2
        val maxRadius = minOf(size.width, size.height) * 0.4f
        val maxIntensity = eulumdatData.maxIntensity.coerceAtLeast(1.0)

        // Draw grid circles (full circle)
        for (i in 1..4) {
            val radius = maxRadius * i / 4
            drawCircle(
                color = onSurfaceColor.copy(alpha = 0.15f),
                radius = radius,
                center = Offset(centerX, centerY),
                style = Stroke(width = 1f)
            )
        }

        // Draw angle lines (full 360°)
        for (angle in 0 until 360 step 30) {
            val rad = angle * PI / 180
            val endX = centerX + (maxRadius * sin(rad)).toFloat()
            val endY = centerY - (maxRadius * cos(rad)).toFloat()
            drawLine(
                color = onSurfaceColor.copy(alpha = 0.15f),
                start = Offset(centerX, centerY),
                end = Offset(endX, endY),
                strokeWidth = 1f
            )
        }

        // Draw C0 plane (right side, gamma 0-180)
        if (eulumdatData.intensities.isNotEmpty() && eulumdatData.gammaAngles.isNotEmpty()) {
            val gammaCount = eulumdatData.gammaCount
            val c0Points = mutableListOf<Offset>()

            for (i in 0 until minOf(gammaCount, eulumdatData.intensities.size)) {
                val gamma = eulumdatData.gammaAngles.getOrNull(i) ?: (i * 180.0 / gammaCount)
                val intensity = eulumdatData.intensities[i]
                val normalizedIntensity = intensity / maxIntensity
                val radius = maxRadius * normalizedIntensity.toFloat()
                val rad = gamma * PI / 180
                val x = centerX + (radius * sin(rad)).toFloat()
                val y = centerY - (radius * cos(rad)).toFloat()
                c0Points.add(Offset(x, y))
            }

            // Draw C0 curve
            for (i in 0 until c0Points.size - 1) {
                drawLine(
                    color = primaryColor,
                    start = c0Points[i],
                    end = c0Points[i + 1],
                    strokeWidth = 3f
                )
            }

            // Draw C180 plane (left side, mirrored) - use second half of data if available
            val cPlaneCount = eulumdatData.cPlaneCount
            val c180Points = mutableListOf<Offset>()

            // Try to get C180 data (typically at index cPlaneCount/2)
            val c180StartIndex = if (cPlaneCount > 1) {
                (cPlaneCount / 2) * gammaCount
            } else {
                0 // Use same data mirrored for symmetric luminaires
            }

            for (i in 0 until minOf(gammaCount, eulumdatData.intensities.size)) {
                val gamma = eulumdatData.gammaAngles.getOrNull(i) ?: (i * 180.0 / gammaCount)
                val dataIndex = c180StartIndex + i
                val intensity = if (dataIndex < eulumdatData.intensities.size) {
                    eulumdatData.intensities[dataIndex]
                } else {
                    eulumdatData.intensities.getOrNull(i) ?: 0.0 // Fallback to C0 data
                }
                val normalizedIntensity = intensity / maxIntensity
                val radius = maxRadius * normalizedIntensity.toFloat()
                val rad = gamma * PI / 180
                // Mirror to left side (negative x)
                val x = centerX - (radius * sin(rad)).toFloat()
                val y = centerY - (radius * cos(rad)).toFloat()
                c180Points.add(Offset(x, y))
            }

            // Draw C180 curve
            for (i in 0 until c180Points.size - 1) {
                drawLine(
                    color = secondaryColor,
                    start = c180Points[i],
                    end = c180Points[i + 1],
                    strokeWidth = 3f
                )
            }
        }

        // Draw labels
        drawContext.canvas.nativeCanvas.apply {
            val paint = android.graphics.Paint().apply {
                color = android.graphics.Color.GRAY
                textSize = 28f
                textAlign = android.graphics.Paint.Align.CENTER
            }
            drawText("0°", centerX, centerY - maxRadius - 15, paint)
            drawText("90°", centerX + maxRadius + 35, centerY + 10, paint)
            drawText("180°", centerX, centerY + maxRadius + 30, paint)
            drawText("270°", centerX - maxRadius - 35, centerY + 10, paint)

            // Legend
            paint.textSize = 24f
            paint.textAlign = android.graphics.Paint.Align.LEFT
            paint.color = android.graphics.Color.rgb(
                (primaryColor.red * 255).toInt(),
                (primaryColor.green * 255).toInt(),
                (primaryColor.blue * 255).toInt()
            )
            drawText("C0", 20f, size.height - 50, paint)
            paint.color = android.graphics.Color.rgb(
                (secondaryColor.red * 255).toInt(),
                (secondaryColor.green * 255).toInt(),
                (secondaryColor.blue * 255).toInt()
            )
            drawText("C180", 20f, size.height - 20, paint)
        }
    }
}

@Composable
fun CartesianDiagram(eulumdatData: EulumdatData, modifier: Modifier = Modifier) {
    val primaryColor = MaterialTheme.colorScheme.primary
    val secondaryColor = MaterialTheme.colorScheme.tertiary
    val onSurfaceColor = MaterialTheme.colorScheme.onSurface

    Canvas(modifier = modifier) {
        val padding = 60f
        val chartWidth = size.width - padding * 2
        val chartHeight = size.height - padding * 2
        val maxIntensity = eulumdatData.maxIntensity.coerceAtLeast(1.0)

        // Draw axes
        // X axis
        drawLine(
            color = onSurfaceColor,
            start = Offset(padding, size.height - padding),
            end = Offset(size.width - padding, size.height - padding),
            strokeWidth = 2f
        )
        // Y axis
        drawLine(
            color = onSurfaceColor,
            start = Offset(padding, padding),
            end = Offset(padding, size.height - padding),
            strokeWidth = 2f
        )

        // Draw grid lines
        for (i in 0..4) {
            val y = size.height - padding - (chartHeight * i / 4)
            drawLine(
                color = onSurfaceColor.copy(alpha = 0.15f),
                start = Offset(padding, y),
                end = Offset(size.width - padding, y),
                strokeWidth = 1f
            )
        }
        for (angle in listOf(0, 30, 60, 90, 120, 150, 180)) {
            val x = padding + (chartWidth * angle / 180)
            drawLine(
                color = onSurfaceColor.copy(alpha = 0.15f),
                start = Offset(x, padding),
                end = Offset(x, size.height - padding),
                strokeWidth = 1f
            )
        }

        // Draw C0 intensity curve
        if (eulumdatData.intensities.isNotEmpty() && eulumdatData.gammaAngles.isNotEmpty()) {
            val gammaCount = eulumdatData.gammaCount
            val c0Points = mutableListOf<Offset>()

            for (i in 0 until minOf(gammaCount, eulumdatData.intensities.size)) {
                val gamma = eulumdatData.gammaAngles.getOrNull(i) ?: (i * 180.0 / gammaCount)
                val intensity = eulumdatData.intensities[i]
                val normalizedIntensity = intensity / maxIntensity
                val x = padding + (chartWidth * gamma / 180).toFloat()
                val y = size.height - padding - (chartHeight * normalizedIntensity).toFloat()
                c0Points.add(Offset(x, y))
            }

            for (i in 0 until c0Points.size - 1) {
                drawLine(
                    color = primaryColor,
                    start = c0Points[i],
                    end = c0Points[i + 1],
                    strokeWidth = 3f
                )
            }

            // Draw C180 curve
            val cPlaneCount = eulumdatData.cPlaneCount
            val c180StartIndex = if (cPlaneCount > 1) (cPlaneCount / 2) * gammaCount else 0
            val c180Points = mutableListOf<Offset>()

            for (i in 0 until minOf(gammaCount, eulumdatData.intensities.size)) {
                val gamma = eulumdatData.gammaAngles.getOrNull(i) ?: (i * 180.0 / gammaCount)
                val dataIndex = c180StartIndex + i
                val intensity = if (dataIndex < eulumdatData.intensities.size) {
                    eulumdatData.intensities[dataIndex]
                } else {
                    eulumdatData.intensities.getOrNull(i) ?: 0.0
                }
                val normalizedIntensity = intensity / maxIntensity
                val x = padding + (chartWidth * gamma / 180).toFloat()
                val y = size.height - padding - (chartHeight * normalizedIntensity).toFloat()
                c180Points.add(Offset(x, y))
            }

            for (i in 0 until c180Points.size - 1) {
                drawLine(
                    color = secondaryColor,
                    start = c180Points[i],
                    end = c180Points[i + 1],
                    strokeWidth = 3f
                )
            }
        }

        // Draw labels
        drawContext.canvas.nativeCanvas.apply {
            val paint = android.graphics.Paint().apply {
                color = android.graphics.Color.GRAY
                textSize = 24f
                textAlign = android.graphics.Paint.Align.CENTER
            }
            // X axis labels
            for (angle in listOf(0, 30, 60, 90, 120, 150, 180)) {
                val x = padding + (chartWidth * angle / 180)
                drawText("$angle°", x, size.height - padding + 30, paint)
            }
            // Y axis label
            paint.textAlign = android.graphics.Paint.Align.RIGHT
            drawText("cd/klm", padding - 10, padding - 10, paint)

            // Legend
            paint.textSize = 24f
            paint.textAlign = android.graphics.Paint.Align.LEFT
            paint.color = android.graphics.Color.rgb(
                (primaryColor.red * 255).toInt(),
                (primaryColor.green * 255).toInt(),
                (primaryColor.blue * 255).toInt()
            )
            drawText("C0", size.width - 100, 30f, paint)
            paint.color = android.graphics.Color.rgb(
                (secondaryColor.red * 255).toInt(),
                (secondaryColor.green * 255).toInt(),
                (secondaryColor.blue * 255).toInt()
            )
            drawText("C180", size.width - 100, 60f, paint)
        }
    }
}

@Composable
fun L3dViewer(engine: GldfEngine, file: GldfFile) {
    var l3dFile by remember { mutableStateOf<L3dFile?>(null) }
    var objData by remember { mutableStateOf<ObjMeshData?>(null) }
    var error by remember { mutableStateOf<String?>(null) }
    var isLoading by remember { mutableStateOf(true) }
    var rotationY by remember { mutableStateOf(30f) }
    var rotationX by remember { mutableStateOf(20f) }

    LaunchedEffect(file.id) {
        try {
            val content = engine.getFileContent(file.id)
            l3dFile = parseL3d(content.data)

            // Parse first OBJ file for 3D view
            l3dFile?.let { l3d ->
                val objAsset = l3d.assets.find { it.name.endsWith(".obj", ignoreCase = true) }
                objAsset?.let { asset ->
                    val objContent = asset.data.map { it.toByte() }.toByteArray().decodeToString()
                    objData = parseObjFile(objContent)
                }
            }
        } catch (e: Exception) {
            error = e.message
        }
        isLoading = false
    }

    when {
        isLoading -> Box(modifier = Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
            CircularProgressIndicator()
        }
        error != null -> Box(modifier = Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
            Column(horizontalAlignment = Alignment.CenterHorizontally) {
                Icon(Icons.Default.Warning, contentDescription = "Error", tint = MaterialTheme.colorScheme.error)
                Text(error ?: "Unknown error", color = MaterialTheme.colorScheme.error)
            }
        }
        l3dFile != null -> {
            Column(modifier = Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(16.dp)) {
                // 3D Preview
                Card(modifier = Modifier.fillMaxWidth()) {
                    Column(modifier = Modifier.padding(16.dp)) {
                        Text("3D Preview", fontWeight = FontWeight.Bold, fontSize = 18.sp)
                        Spacer(modifier = Modifier.height(8.dp))

                        if (objData != null && objData!!.vertices.isNotEmpty()) {
                            Wireframe3DView(
                                objData = objData!!,
                                rotationX = rotationX,
                                rotationY = rotationY,
                                modifier = Modifier.fillMaxWidth().height(300.dp)
                            )
                            Spacer(modifier = Modifier.height(8.dp))

                            // Rotation controls
                            Text("Rotate Y", fontSize = 12.sp, color = MaterialTheme.colorScheme.onSurfaceVariant)
                            Slider(
                                value = rotationY,
                                onValueChange = { rotationY = it },
                                valueRange = 0f..360f
                            )
                            Text("Rotate X", fontSize = 12.sp, color = MaterialTheme.colorScheme.onSurfaceVariant)
                            Slider(
                                value = rotationX,
                                onValueChange = { rotationX = it },
                                valueRange = -90f..90f
                            )
                        } else {
                            Box(
                                modifier = Modifier.fillMaxWidth().height(200.dp),
                                contentAlignment = Alignment.Center
                            ) {
                                Text("No OBJ geometry found", color = MaterialTheme.colorScheme.onSurfaceVariant)
                            }
                        }
                    }
                }

                Spacer(modifier = Modifier.height(16.dp))

                // Info card
                Card(modifier = Modifier.fillMaxWidth()) {
                    Column(modifier = Modifier.padding(16.dp)) {
                        Text("Geometry Info", fontWeight = FontWeight.Bold, fontSize = 18.sp)
                        Spacer(modifier = Modifier.height(8.dp))
                        InfoRow("Created With", l3dFile!!.scene.createdWithApplication)
                        InfoRow("Parts", l3dFile!!.scene.parts.size.toString())
                        InfoRow("Joints", l3dFile!!.scene.joints.size.toString())
                        InfoRow("Assets", l3dFile!!.assets.size.toString())
                        objData?.let {
                            InfoRow("Vertices", it.vertices.size.toString())
                            InfoRow("Faces", it.faces.size.toString())
                        }
                    }
                }

                Spacer(modifier = Modifier.height(16.dp))

                // Geometry definitions
                if (l3dFile!!.scene.geometryDefinitions.isNotEmpty()) {
                    Text("Geometry Files", fontWeight = FontWeight.Bold, fontSize = 16.sp)
                    Spacer(modifier = Modifier.height(8.dp))
                    l3dFile!!.scene.geometryDefinitions.forEach { geoDef ->
                        Card(modifier = Modifier.fillMaxWidth().padding(vertical = 4.dp)) {
                            Column(modifier = Modifier.padding(12.dp)) {
                                Text(geoDef.filename, fontWeight = FontWeight.Medium)
                                Text("ID: ${geoDef.id}", fontSize = 12.sp, color = MaterialTheme.colorScheme.onSurfaceVariant)
                                Text("Units: ${geoDef.units}", fontSize = 12.sp, color = MaterialTheme.colorScheme.onSurfaceVariant)
                            }
                        }
                    }
                }

                Spacer(modifier = Modifier.height(16.dp))

                // Scene parts
                if (l3dFile!!.scene.parts.isNotEmpty()) {
                    Text("Scene Parts", fontWeight = FontWeight.Bold, fontSize = 16.sp)
                    Spacer(modifier = Modifier.height(8.dp))
                    l3dFile!!.scene.parts.forEach { part ->
                        Card(modifier = Modifier.fillMaxWidth().padding(vertical = 4.dp)) {
                            Column(modifier = Modifier.padding(12.dp)) {
                                Text(part.partName, fontWeight = FontWeight.Medium)
                                Text("Geometry: ${part.geometryId}", fontSize = 12.sp, color = MaterialTheme.colorScheme.onSurfaceVariant)
                                Text(
                                    "Position: (${part.position.x.format()}, ${part.position.y.format()}, ${part.position.z.format()})",
                                    fontSize = 12.sp,
                                    color = MaterialTheme.colorScheme.onSurfaceVariant
                                )
                                if (part.lightEmittingObjects.isNotEmpty()) {
                                    Text(
                                        "Light Emitting Objects: ${part.lightEmittingObjects.size}",
                                        fontSize = 12.sp,
                                        color = MaterialTheme.colorScheme.primary
                                    )
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

// Simple OBJ parser data structures
data class ObjVertex(val x: Float, val y: Float, val z: Float)
data class ObjFace(val vertexIndices: List<Int>)
data class ObjMeshData(val vertices: List<ObjVertex>, val faces: List<ObjFace>)

fun parseObjFile(content: String): ObjMeshData {
    val vertices = mutableListOf<ObjVertex>()
    val faces = mutableListOf<ObjFace>()

    content.lines().forEach { line ->
        val parts = line.trim().split("\\s+".toRegex())
        when {
            parts.isNotEmpty() && parts[0] == "v" && parts.size >= 4 -> {
                val x = parts[1].toFloatOrNull() ?: 0f
                val y = parts[2].toFloatOrNull() ?: 0f
                val z = parts[3].toFloatOrNull() ?: 0f
                vertices.add(ObjVertex(x, y, z))
            }
            parts.isNotEmpty() && parts[0] == "f" && parts.size >= 4 -> {
                val indices = parts.drop(1).mapNotNull { part ->
                    // Handle formats: "1", "1/2", "1/2/3", "1//3"
                    val idx = part.split("/")[0].toIntOrNull()
                    idx?.minus(1) // OBJ indices are 1-based
                }.filter { it >= 0 }
                if (indices.size >= 3) {
                    faces.add(ObjFace(indices))
                }
            }
        }
    }

    return ObjMeshData(vertices, faces)
}

@Composable
fun Wireframe3DView(
    objData: ObjMeshData,
    rotationX: Float,
    rotationY: Float,
    modifier: Modifier = Modifier
) {
    val primaryColor = MaterialTheme.colorScheme.primary
    val secondaryColor = MaterialTheme.colorScheme.tertiary
    val onSurfaceColor = MaterialTheme.colorScheme.onSurface

    Canvas(modifier = modifier) {
        val centerX = size.width / 2
        val centerY = size.height / 2

        // Calculate bounding box to normalize
        if (objData.vertices.isEmpty()) return@Canvas

        var minX = Float.MAX_VALUE; var maxX = Float.MIN_VALUE
        var minY = Float.MAX_VALUE; var maxY = Float.MIN_VALUE
        var minZ = Float.MAX_VALUE; var maxZ = Float.MIN_VALUE

        objData.vertices.forEach { v ->
            minX = minOf(minX, v.x); maxX = maxOf(maxX, v.x)
            minY = minOf(minY, v.y); maxY = maxOf(maxY, v.y)
            minZ = minOf(minZ, v.z); maxZ = maxOf(maxZ, v.z)
        }

        val scaleX = if (maxX - minX > 0) maxX - minX else 1f
        val scaleY = if (maxY - minY > 0) maxY - minY else 1f
        val scaleZ = if (maxZ - minZ > 0) maxZ - minZ else 1f
        val maxScale = maxOf(scaleX, scaleY, scaleZ)
        val scale = minOf(size.width, size.height) * 0.35f / maxScale

        val offsetX = (minX + maxX) / 2
        val offsetY = (minY + maxY) / 2
        val offsetZ = (minZ + maxZ) / 2

        // Rotation angles in radians
        val rx = rotationX * PI.toFloat() / 180f
        val ry = rotationY * PI.toFloat() / 180f

        // Transform and project vertices
        fun project(v: ObjVertex): Offset {
            // Center the model
            var x = v.x - offsetX
            var y = v.y - offsetY
            var z = v.z - offsetZ

            // Rotate around Y axis
            val cosY = cos(ry)
            val sinY = sin(ry)
            val x1 = x * cosY - z * sinY
            val z1 = x * sinY + z * cosY
            x = x1
            z = z1

            // Rotate around X axis
            val cosX = cos(rx)
            val sinX = sin(rx)
            val y1 = y * cosX - z * sinX
            val z2 = y * sinX + z * cosX
            y = y1

            // Simple orthographic projection
            val screenX = centerX + x * scale
            val screenY = centerY - y * scale

            return Offset(screenX, screenY)
        }

        // Draw coordinate axes
        val axisLength = minOf(size.width, size.height) * 0.1f
        val origin = Offset(size.width - 60, size.height - 60)

        // X axis (red)
        val xEnd = Offset(
            origin.x + axisLength * cos(ry),
            origin.y - axisLength * sin(rx) * sin(ry)
        )
        drawLine(Color.Red, origin, xEnd, strokeWidth = 2f)

        // Y axis (green)
        val yEnd = Offset(
            origin.x,
            origin.y - axisLength * cos(rx)
        )
        drawLine(Color.Green, origin, yEnd, strokeWidth = 2f)

        // Z axis (blue)
        val zEnd = Offset(
            origin.x - axisLength * sin(ry),
            origin.y - axisLength * sin(rx) * cos(ry)
        )
        drawLine(Color.Blue, origin, zEnd, strokeWidth = 2f)

        // Draw edges
        val drawnEdges = mutableSetOf<Pair<Int, Int>>()

        objData.faces.forEach { face ->
            val indices = face.vertexIndices
            for (i in indices.indices) {
                val i1 = indices[i]
                val i2 = indices[(i + 1) % indices.size]

                // Avoid drawing edges twice
                val edge = if (i1 < i2) Pair(i1, i2) else Pair(i2, i1)
                if (edge in drawnEdges) return@forEach
                drawnEdges.add(edge)

                if (i1 < objData.vertices.size && i2 < objData.vertices.size) {
                    val p1 = project(objData.vertices[i1])
                    val p2 = project(objData.vertices[i2])

                    drawLine(
                        color = primaryColor.copy(alpha = 0.7f),
                        start = p1,
                        end = p2,
                        strokeWidth = 1.5f
                    )
                }
            }
        }

        // Draw vertex count info
        drawContext.canvas.nativeCanvas.apply {
            val paint = android.graphics.Paint().apply {
                color = android.graphics.Color.GRAY
                textSize = 24f
                textAlign = android.graphics.Paint.Align.LEFT
            }
            drawText("${objData.vertices.size} vertices", 10f, 30f, paint)
            drawText("${objData.faces.size} faces", 10f, 60f, paint)
        }
    }
}

fun Double.format(): String = String.format("%.2f", this)

@Composable
fun GenericFileViewer(engine: GldfEngine, file: GldfFile) {
    var content by remember { mutableStateOf<String?>(null) }
    var error by remember { mutableStateOf<String?>(null) }
    var isLoading by remember { mutableStateOf(true) }

    LaunchedEffect(file.id) {
        try {
            content = engine.getFileContentAsString(file.id)
        } catch (e: Exception) {
            error = e.message
        }
        isLoading = false
    }

    when {
        isLoading -> Box(modifier = Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
            CircularProgressIndicator()
        }
        error != null -> Box(modifier = Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
            Column(horizontalAlignment = Alignment.CenterHorizontally) {
                Icon(Icons.Default.Warning, contentDescription = "Error", tint = MaterialTheme.colorScheme.error)
                Spacer(modifier = Modifier.height(8.dp))
                Text(error ?: "Unknown error", color = MaterialTheme.colorScheme.error)
            }
        }
        content != null -> {
            Column(modifier = Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(16.dp)) {
                Text(
                    text = content!!,
                    fontFamily = androidx.compose.ui.text.font.FontFamily.Monospace,
                    fontSize = 10.sp
                )
            }
        }
    }
}

@Composable
fun LightSourcesTab(engine: GldfEngine) {
    val lightSources = engine.getLightSources()
    val variants = engine.getVariants()

    LazyColumn(modifier = Modifier.fillMaxSize()) {
        item {
            Text("Light Sources", fontWeight = FontWeight.Bold, fontSize = 18.sp, modifier = Modifier.padding(16.dp))
        }
        items(lightSources) { ls ->
            ListItem(
                headlineContent = { Text(ls.name.ifEmpty { ls.id }) },
                supportingContent = { Text("ID: ${ls.id}") },
                leadingContent = { Icon(Icons.Default.Star, contentDescription = null) },
                trailingContent = {
                    AssistChip(onClick = {}, label = { Text(ls.lightSourceType) })
                }
            )
        }
        if (variants.isNotEmpty()) {
            item {
                Text("Variants", fontWeight = FontWeight.Bold, fontSize = 18.sp, modifier = Modifier.padding(16.dp))
            }
            items(variants) { variant ->
                ListItem(
                    headlineContent = { Text(variant.name.ifEmpty { variant.id }) },
                    supportingContent = {
                        Text(if (variant.description.isNotEmpty()) variant.description else "ID: ${variant.id}")
                    },
                    leadingContent = { Icon(Icons.Default.Check, contentDescription = null) }
                )
            }
        }
    }
}

@Composable
fun RawTab(engine: GldfEngine) {
    var rawXml by remember { mutableStateOf<String?>(null) }
    var isLoading by remember { mutableStateOf(true) }

    LaunchedEffect(Unit) {
        try {
            rawXml = engine.toXml()
        } catch (e: Exception) {
            rawXml = "Error: ${e.message}"
        }
        isLoading = false
    }

    if (isLoading) {
        Box(modifier = Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
            CircularProgressIndicator()
        }
    } else {
        Column(
            modifier = Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(16.dp)
        ) {
            Text(
                text = rawXml ?: "",
                fontFamily = androidx.compose.ui.text.font.FontFamily.Monospace,
                fontSize = 10.sp
            )
        }
    }
}

private fun loadGldfFile(
    context: android.content.Context,
    uri: Uri,
    onResult: (Result<GldfEngine>) -> Unit
) {
    Thread {
        try {
            val inputStream = context.contentResolver.openInputStream(uri)
            val bytes = inputStream?.readBytes() ?: throw Exception("Cannot read file")
            inputStream.close()

            val engine = GldfEngine.fromBytes(bytes)

            android.os.Handler(android.os.Looper.getMainLooper()).post {
                onResult(Result.success(engine))
            }
        } catch (e: Exception) {
            android.os.Handler(android.os.Looper.getMainLooper()).post {
                onResult(Result.failure(e))
            }
        }
    }.start()
}
