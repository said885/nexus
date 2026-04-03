package com.nexus.messenger

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.Call
import androidx.compose.material.icons.filled.Check
import androidx.compose.material.icons.filled.Lock
import androidx.compose.material.icons.filled.Send
import androidx.compose.material.icons.filled.Settings
import androidx.compose.material.icons.filled.Videocam
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FloatingActionButton
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.material3.darkColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.navigation.NavHostController
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.rememberNavController
import com.nexus.messenger.ui.screen.CallScreen
import com.nexus.messenger.ui.screen.ChatScreen
import com.nexus.messenger.ui.screen.ConversationListScreen
import com.nexus.messenger.ui.screen.ProfileScreen
import com.nexus.messenger.ui.screen.SettingsScreen
import com.nexus.messenger.vm.ChatViewModel
import com.nexus.messenger.vm.ConversationsViewModel
import com.nexus.messenger.vm.SetupState
import com.nexus.messenger.vm.SetupViewModel
import org.koin.androidx.compose.koinViewModel

// Dark theme colors for NEXUS
private val NexusDarkColorScheme = darkColorScheme(
    primary = Color(0xFF6C63FF),
    secondary = Color(0xFF03DAC6),
    tertiary = Color(0xFFBB86FC),
    background = Color(0xFF121212),
    surface = Color(0xFF1E1E1E),
    onPrimary = Color.White,
    onSecondary = Color.Black,
    onTertiary = Color.Black,
    onBackground = Color(0xFFE0E0E0),
    onSurface = Color(0xFFE0E0E0),
)

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        setContent {
            NexusTheme {
                Surface(
                    modifier = Modifier.fillMaxSize(),
                    color = MaterialTheme.colorScheme.background
                ) {
                    NexusNavHost()
                }
            }
        }
    }
}

@Composable
fun NexusTheme(content: @Composable () -> Unit) {
    MaterialTheme(
        colorScheme = NexusDarkColorScheme,
        content = content
    )
}

@Composable
fun NexusNavHost(
    navController: NavHostController = rememberNavController()
) {
    val setupViewModel: SetupViewModel = koinViewModel()
    val conversationsViewModel: ConversationsViewModel = koinViewModel()

    val isSetupComplete by setupViewModel.isSetupComplete.collectAsState()

    NavHost(
        navController = navController,
        startDestination = if (isSetupComplete) "conversations" else "setup"
    ) {
        composable("setup") {
            SetupScreen(
                viewModel = setupViewModel,
                onSetupComplete = {
                    navController.navigate("conversations") {
                        popUpTo("setup") { inclusive = true }
                    }
                }
            )
        }

        composable("conversations") {
            ConversationListScreen(
                viewModel = conversationsViewModel,
                onConversationClick = { conversationId, participantHash ->
                    navController.navigate("chat/$conversationId/$participantHash")
                },
                onNewConversation = {
                    navController.navigate("new_conversation")
                },
                onSettingsClick = {
                    navController.navigate("settings")
                }
            )
        }

        composable("chat/{conversationId}/{participantHash}") { backStackEntry ->
            val conversationId = backStackEntry.arguments?.getString("conversationId")?.toLongOrNull() ?: 0L
            val participantHash = backStackEntry.arguments?.getString("participantHash") ?: ""
            val chatViewModel: ChatViewModel = koinViewModel(
                parameters = { org.koin.core.parameter.parametersOf(conversationId, participantHash) }
            )
            ChatScreen(
                viewModel = chatViewModel,
                onBackClick = { navController.popBackStack() },
                onCallClick = { isVideo ->
                    navController.navigate("call/$participantHash/$isVideo")
                }
            )
        }

        composable("call/{participantHash}/{isVideo}") { backStackEntry ->
            val participantHash = backStackEntry.arguments?.getString("participantHash") ?: ""
            val isVideo = backStackEntry.arguments?.getString("isVideo")?.toBoolean() ?: false
            CallScreen(
                participantHash = participantHash,
                isVideo = isVideo,
                onEndCall = { navController.popBackStack() }
            )
        }

        composable("settings") {
            SettingsScreen(
                onBackClick = { navController.popBackStack() },
                onProfileClick = { navController.navigate("profile") }
            )
        }

        composable("profile") {
            ProfileScreen(
                onBackClick = { navController.popBackStack() }
            )
        }

        composable("new_conversation") {
            NewConversationScreen(
                onBackClick = { navController.popBackStack() },
                onConversationCreated = { conversationId, participantHash ->
                    navController.navigate("chat/$conversationId/$participantHash") {
                        popUpTo("conversations")
                    }
                }
            )
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SetupScreen(
    viewModel: SetupViewModel,
    onSetupComplete: () -> Unit
) {
    val state by viewModel.state.collectAsState()
    var relayUrl by remember { mutableStateOf("ws://localhost:8443/ws") }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("NEXUS Setup") },
                colors = TopAppBarDefaults.topAppBarColors(
                    containerColor = MaterialTheme.colorScheme.surface
                )
            )
        }
    ) { padding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
                .padding(24.dp)
                .verticalScroll(rememberScrollState()),
            horizontalAlignment = Alignment.CenterHorizontally
        ) {
            Spacer(modifier = Modifier.height(32.dp))

            // Logo
            Box(
                modifier = Modifier
                    .size(120.dp)
                    .clip(CircleShape)
                    .background(
                        Brush.linearGradient(
                            colors = listOf(
                                MaterialTheme.colorScheme.primary,
                                MaterialTheme.colorScheme.secondary
                            )
                        )
                    ),
                contentAlignment = Alignment.Center
            ) {
                Icon(
                    imageVector = Icons.Default.Lock,
                    contentDescription = "NEXUS",
                    modifier = Modifier.size(64.dp),
                    tint = Color.White
                )
            }

            Spacer(modifier = Modifier.height(24.dp))

            Text(
                text = "NEXUS Messenger",
                style = MaterialTheme.typography.headlineMedium,
                fontWeight = FontWeight.Bold
            )

            Text(
                text = "Post-Quantum Secure Messaging",
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.7f)
            )

            Spacer(modifier = Modifier.height(48.dp))

            // Setup steps
            when (val currentState = state) {
                is SetupState.Idle -> {
                    SetupStep(
                        step = 1,
                        title = "Generate Identity",
                        description = "Create your post-quantum cryptographic identity",
                        isActive = true
                    )

                    Spacer(modifier = Modifier.height(16.dp))

                    Button(
                        onClick = { viewModel.generateIdentity() },
                        modifier = Modifier.fillMaxWidth()
                    ) {
                        Icon(Icons.Default.Lock, contentDescription = null)
                        Spacer(modifier = Modifier.width(8.dp))
                        Text("Generate Identity")
                    }
                }

                is SetupState.GeneratingIdentity -> {
                    Column(
                        horizontalAlignment = Alignment.CenterHorizontally
                    ) {
                        CircularProgressIndicator()
                        Spacer(modifier = Modifier.height(16.dp))
                        Text("Generating your cryptographic identity...")
                        Text(
                            text = "This may take a moment",
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.7f)
                        )
                    }
                }

                is SetupState.IdentityGenerated -> {
                    Card(
                        modifier = Modifier.fillMaxWidth(),
                        colors = CardDefaults.cardColors(
                            containerColor = MaterialTheme.colorScheme.primaryContainer
                        )
                    ) {
                        Column(
                            modifier = Modifier.padding(16.dp)
                        ) {
                            Row(
                                verticalAlignment = Alignment.CenterVertically
                            ) {
                                Icon(
                                    Icons.Default.Check,
                                    contentDescription = null,
                                    tint = MaterialTheme.colorScheme.primary
                                )
                                Spacer(modifier = Modifier.width(8.dp))
                                Text(
                                    text = "Identity Generated",
                                    style = MaterialTheme.typography.titleMedium,
                                    fontWeight = FontWeight.Bold
                                )
                            }

                            Spacer(modifier = Modifier.height(12.dp))

                            Text(
                                text = "Fingerprint:",
                                style = MaterialTheme.typography.bodySmall
                            )
                            Text(
                                text = currentState.fingerprint,
                                fontFamily = FontFamily.Monospace,
                                style = MaterialTheme.typography.bodyLarge
                            )

                            Spacer(modifier = Modifier.height(8.dp))

                            Text(
                                text = "Identity Hash:",
                                style = MaterialTheme.typography.bodySmall
                            )
                            Text(
                                text = currentState.identityHashHex.take(16) + "...",
                                fontFamily = FontFamily.Monospace,
                                style = MaterialTheme.typography.bodyMedium
                            )
                        }
                    }

                    Spacer(modifier = Modifier.height(16.dp))

                    Button(
                        onClick = { viewModel.proceedToRelayConfig() },
                        modifier = Modifier.fillMaxWidth()
                    ) {
                        Text("Configure Relay")
                    }
                }

                is SetupState.ConfiguringRelay -> {
                    SetupStep(
                        step = 2,
                        title = "Configure Relay Server",
                        description = "Connect to a NEXUS relay server",
                        isActive = true
                    )

                    Spacer(modifier = Modifier.height(16.dp))

                    OutlinedTextField(
                        value = relayUrl,
                        onValueChange = { relayUrl = it },
                        label = { Text("Relay Server URL") },
                        modifier = Modifier.fillMaxWidth(),
                        singleLine = true
                    )

                    Spacer(modifier = Modifier.height(8.dp))

                    Text(
                        text = "Default: ws://localhost:8443/ws",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.5f)
                    )

                    Spacer(modifier = Modifier.height(16.dp))

                    Button(
                        onClick = {
                            viewModel.saveRelayUrl(relayUrl)
                            viewModel.completeSetup()
                        },
                        modifier = Modifier.fillMaxWidth()
                    ) {
                        Text("Complete Setup")
                    }
                }

                is SetupState.Complete -> {
                    LaunchedEffect(Unit) {
                        onSetupComplete()
                    }
                }

                is SetupState.Error -> {
                    Card(
                        modifier = Modifier.fillMaxWidth(),
                        colors = CardDefaults.cardColors(
                            containerColor = MaterialTheme.colorScheme.errorContainer
                        )
                    ) {
                        Column(
                            modifier = Modifier.padding(16.dp)
                        ) {
                            Text(
                                text = "Error",
                                style = MaterialTheme.typography.titleMedium,
                                color = MaterialTheme.colorScheme.error
                            )
                            Spacer(modifier = Modifier.height(8.dp))
                            Text(
                                text = currentState.message,
                                color = MaterialTheme.colorScheme.onErrorContainer
                            )
                        }
                    }

                    Spacer(modifier = Modifier.height(16.dp))

                    Button(
                        onClick = { viewModel.generateIdentity() },
                        modifier = Modifier.fillMaxWidth()
                    ) {
                        Text("Retry")
                    }
                }

                else -> {}
            }
        }
    }
}

@Composable
fun SetupStep(
    step: Int,
    title: String,
    description: String,
    isActive: Boolean
) {
    Card(
        modifier = Modifier.fillMaxWidth(),
        colors = CardDefaults.cardColors(
            containerColor = if (isActive)
                MaterialTheme.colorScheme.primaryContainer
            else
                MaterialTheme.colorScheme.surfaceVariant
        )
    ) {
        Row(
            modifier = Modifier.padding(16.dp),
            verticalAlignment = Alignment.CenterVertically
        ) {
            Box(
                modifier = Modifier
                    .size(40.dp)
                    .clip(CircleShape)
                    .background(
                        if (isActive) MaterialTheme.colorScheme.primary
                        else MaterialTheme.colorScheme.outline
                    ),
                contentAlignment = Alignment.Center
            ) {
                Text(
                    text = step.toString(),
                    color = Color.White,
                    fontWeight = FontWeight.Bold
                )
            }

            Spacer(modifier = Modifier.width(16.dp))

            Column {
                Text(
                    text = title,
                    style = MaterialTheme.typography.titleMedium,
                    fontWeight = FontWeight.Bold
                )
                Text(
                    text = description,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.7f)
                )
            }
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun NewConversationScreen(
    onBackClick: () -> Unit,
    onConversationCreated: (Long, String) -> Unit
) {
    var participantHash by remember { mutableStateOf("") }
    var displayName by remember { mutableStateOf("") }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("New Conversation") },
                navigationIcon = {
                    IconButton(onClick = onBackClick) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
                    }
                },
                colors = TopAppBarDefaults.topAppBarColors(
                    containerColor = MaterialTheme.colorScheme.surface
                )
            )
        }
    ) { padding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
                .padding(24.dp)
        ) {
            Text(
                text = "Start a secure conversation",
                style = MaterialTheme.typography.titleMedium
            )

            Spacer(modifier = Modifier.height(24.dp))

            OutlinedTextField(
                value = participantHash,
                onValueChange = { participantHash = it },
                label = { Text("Recipient Identity Hash") },
                modifier = Modifier.fillMaxWidth(),
                singleLine = true,
                placeholder = { Text("64-character hex string") }
            )

            Spacer(modifier = Modifier.height(16.dp))

            OutlinedTextField(
                value = displayName,
                onValueChange = { displayName = it },
                label = { Text("Display Name (optional)") },
                modifier = Modifier.fillMaxWidth(),
                singleLine = true
            )

            Spacer(modifier = Modifier.height(24.dp))

            Button(
                onClick = {
                    if (participantHash.length == 64) {
                        onConversationCreated(0L, participantHash)
                    }
                },
                modifier = Modifier.fillMaxWidth(),
                enabled = participantHash.length == 64
            ) {
                Icon(Icons.Default.Send, contentDescription = null)
                Spacer(modifier = Modifier.width(8.dp))
                Text("Start Conversation")
            }

            if (participantHash.isNotEmpty() && participantHash.length != 64) {
                Spacer(modifier = Modifier.height(8.dp))
                Text(
                    text = "Identity hash must be 64 characters",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.error
                )
            }
        }
    }
}
