// Android Call Screen - Jetpack Compose
// nexus-android/src/main/java/com/nexus/ui/screen/CallScreen.kt

package com.nexus.ui.screen

import android.Manifest
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp

@Composable
fun CallScreen(
    recipientName: String,
    recipientAvatarUrl: String?,
    callType: CallType,
    onEndCall: () -> Unit,
    onMuteMic: (Boolean) -> Unit,
    onToggleCamera: (Boolean) -> Unit,
    modifier: Modifier = Modifier
) {
    var isMuted by remember { mutableStateOf(false) }
    var isCameraOn by remember { mutableStateOf(callType == CallType.VIDEO) }
    var callDuration by remember { mutableStateOf("00:00") }
    
    LaunchedEffect(Unit) {
        // Simulate call timer
        var seconds = 0
        while (true) {
            kotlinx.coroutines.delay(1000)
            seconds++
            val minutes = seconds / 60
            val secs = seconds % 60
            callDuration = String.format("%02d:%02d", minutes, secs)
        }
    }
    
    Box(
        modifier = modifier
            .fillMaxSize()
            .background(Color.Black)
    ) {
        // Remote Video / Avatar
        if (callType == CallType.VIDEO && isCameraOn) {
            // Video preview placeholder
            Box(
                modifier = Modifier.fillMaxSize(),
                contentAlignment = Alignment.Center
            ) {
                Surface(
                    modifier = Modifier
                        .size(100.dp)
                        .clip(CircleShape),
                    color = Color.Gray
                ) {
                    Box(
                        contentAlignment = Alignment.Center,
                        modifier = Modifier.fillMaxSize()
                    ) {
                        Text(
                            text = recipientName.firstOrNull()?.toString() ?: "?",
                            fontSize = 48.sp,
                            fontWeight = FontWeight.Bold,
                            color = Color.White
                        )
                    }
                }
            }
        }
        
        // Call Info Overlay
        Column(
            modifier = Modifier
                .align(Alignment.TopCenter)
                .padding(24.dp),
            horizontalAlignment = Alignment.CenterHorizontally
        ) {
            Text(
                text = recipientName,
                fontSize = 24.sp,
                fontWeight = FontWeight.Bold,
                color = Color.White
            )
            
            Spacer(modifier = Modifier.height(8.dp))
            
            Text(
                text = callDuration,
                fontSize = 16.sp,
                color = Color.Gray
            )
            
            Spacer(modifier = Modifier.height(4.dp))
            
            Text(
                text = if (callType == CallType.VIDEO) "Video Call" else "Voice Call",
                fontSize = 12.sp,
                color = Color.Gray
            )
        }
        
        // Local Video Thumbnail
        if (callType == CallType.VIDEO && isCameraOn) {
            Surface(
                modifier = Modifier
                    .align(Alignment.TopEnd)
                    .padding(16.dp)
                    .size(100.dp)
                    .clip(RoundedCornerShape(8.dp)),
                color = Color.DarkGray
            ) {
                Box(contentAlignment = Alignment.Center) {
                    Text(
                        text = "You",
                        color = Color.White,
                        fontSize = 12.sp
                    )
                }
            }
        }
        
        // Control Buttons
        Row(
            modifier = Modifier
                .align(Alignment.BottomCenter)
                .padding(24.dp)
                .fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceEvenly,
            verticalAlignment = Alignment.CenterVertically
        ) {
            // Mute Button
            Surface(
                modifier = Modifier
                    .size(56.dp)
                    .clip(CircleShape),
                color = if (isMuted) Color.Red else Color.Gray.copy(alpha = 0.3f)
            ) {
                IconButton(
                    onClick = {
                        isMuted = !isMuted
                        onMuteMic(isMuted)
                    }
                ) {
                    Icon(
                        imageVector = if (isMuted) Icons.Default.MicOff else Icons.Default.Mic,
                        contentDescription = "Mute",
                        tint = Color.White,
                        modifier = Modifier.size(24.dp)
                    )
                }
            }
            
            // Camera Toggle (for video calls)
            if (callType == CallType.VIDEO) {
                Surface(
                    modifier = Modifier
                        .size(56.dp)
                        .clip(CircleShape),
                    color = Color.Gray.copy(alpha = 0.3f)
                ) {
                    IconButton(
                        onClick = {
                            isCameraOn = !isCameraOn
                            onToggleCamera(isCameraOn)
                        }
                    ) {
                        Icon(
                            imageVector = if (isCameraOn) Icons.Default.Videocam else Icons.Default.VideocamOff,
                            contentDescription = "Camera",
                            tint = Color.White,
                            modifier = Modifier.size(24.dp)
                        )
                    }
                }
            }
            
            // Speaker Button
            Surface(
                modifier = Modifier
                    .size(56.dp)
                    .clip(CircleShape),
                color = Color.Gray.copy(alpha = 0.3f)
            ) {
                IconButton(onClick = { /* Toggle speaker */ }) {
                    Icon(
                        imageVector = Icons.Default.VolumeUp,
                        contentDescription = "Speaker",
                        tint = Color.White,
                        modifier = Modifier.size(24.dp)
                    )
                }
            }
            
            // End Call Button
            Surface(
                modifier = Modifier
                    .size(56.dp)
                    .clip(CircleShape),
                color = Color.Red
            ) {
                IconButton(onClick = onEndCall) {
                    Icon(
                        imageVector = Icons.Default.Call,
                        contentDescription = "End Call",
                        tint = Color.White,
                        modifier = Modifier.size(24.dp),
                        contentScale = androidx.compose.ui.layout.ContentScale.Inside
                    )
                }
            }
        }
    }
}

enum class CallType {
    VOICE,
    VIDEO
}

@Composable
fun CallIncomingScreen(
    callerName: String,
    callerAvatarUrl: String?,
    onAccept: () -> Unit,
    onReject: () -> Unit,
    modifier: Modifier = Modifier
) {
    Box(
        modifier = modifier
            .fillMaxSize()
            .background(
                brush = androidx.compose.foundation.background.Brush.verticalGradient(
                    colors = listOf(
                        Color(0xFF667eea),
                        Color(0xFF764ba2)
                    )
                )
            )
    ) {
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(24.dp),
            verticalArrangement = Arrangement.Center,
            horizontalAlignment = Alignment.CenterHorizontally
        ) {
            // Avatar
            Surface(
                modifier = Modifier
                    .size(120.dp)
                    .clip(CircleShape),
                color = Color.White.copy(alpha = 0.2f)
            ) {
                Box(
                    contentAlignment = Alignment.Center,
                    modifier = Modifier.fillMaxSize()
                ) {
                    Text(
                        text = callerName.firstOrNull()?.toString() ?: "?",
                        fontSize = 56.sp,
                        fontWeight = FontWeight.Bold,
                        color = Color.White
                    )
                }
            }
            
            Spacer(modifier = Modifier.height(32.dp))
            
            // Caller Info
            Text(
                text = callerName,
                fontSize = 28.sp,
                fontWeight = FontWeight.Bold,
                color = Color.White
            )
            
            Spacer(modifier = Modifier.height(8.dp))
            
            Text(
                text = "Incoming call...",
                fontSize = 16.sp,
                color = Color.White.copy(alpha = 0.8f)
            )
            
            Spacer(modifier = Modifier.height(48.dp))
            
            // Accept / Reject Buttons
            Row(
                modifier = Modifier
                    .fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceEvenly
            ) {
                // Reject Button
                Surface(
                    modifier = Modifier
                        .size(64.dp)
                        .clip(CircleShape),
                    color = Color.Red
                ) {
                    IconButton(onClick = onReject) {
                        Icon(
                            imageVector = Icons.Default.Call,
                            contentDescription = "Reject",
                            tint = Color.White,
                            modifier = Modifier.size(32.dp)
                        )
                    }
                }
                
                // Accept Button
                Surface(
                    modifier = Modifier
                        .size(64.dp)
                        .clip(CircleShape),
                    color = Color.Green
                ) {
                    IconButton(onClick = onAccept) {
                        Icon(
                            imageVector = Icons.Default.Call,
                            contentDescription = "Accept",
                            tint = Color.White,
                            modifier = Modifier.size(32.dp)
                        )
                    }
                }
            }
        }
    }
}

@Composable
fun CallOutgoingScreen(
    recipientName: String,
    onCancel: () -> Unit,
    modifier: Modifier = Modifier
) {
    Box(
        modifier = modifier
            .fillMaxSize()
            .background(Color.Black)
    ) {
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(24.dp),
            verticalArrangement = Arrangement.Center,
            horizontalAlignment = Alignment.CenterHorizontally
        ) {
            // Avatar
            Surface(
                modifier = Modifier
                    .size(120.dp)
                    .clip(CircleShape),
                color = Color.Gray
            ) {
                Box(
                    contentAlignment = Alignment.Center,
                    modifier = Modifier.fillMaxSize()
                ) {
                    Text(
                        text = recipientName.firstOrNull()?.toString() ?: "?",
                        fontSize = 56.sp,
                        fontWeight = FontWeight.Bold,
                        color = Color.White
                    )
                }
            }
            
            Spacer(modifier = Modifier.height(32.dp))
            
            // Info
            Text(
                text = "Calling $recipientName...",
                fontSize = 20.sp,
                fontWeight = FontWeight.SemiBold,
                color = Color.White
            )
            
            Spacer(modifier = Modifier.height(48.dp))
            
            // Ringing animation
            LazyRow(
                modifier = Modifier
                    .height(40.dp),
                horizontalArrangement = Arrangement.Center,
                verticalAlignment = Alignment.CenterVertically
            ) {
                items(3) { index ->
                    Box(
                        modifier = Modifier
                            .size(8.dp)
                            .background(Color.White.copy(alpha = 0.5f), CircleShape)
                            .padding(4.dp)
                    )
                }
            }
            
            Spacer(modifier = Modifier.height(64.dp))
            
            // Cancel Button
            Surface(
                modifier = Modifier
                    .size(64.dp)
                    .clip(CircleShape),
                color = Color.Red
            ) {
                IconButton(onClick = onCancel) {
                    Icon(
                        imageVector = Icons.Default.Call,
                        contentDescription = "Cancel",
                        tint = Color.White,
                        modifier = Modifier.size(32.dp)
                    )
                }
            }
        }
    }
}
