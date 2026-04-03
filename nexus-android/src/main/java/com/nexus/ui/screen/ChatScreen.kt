// Android Chat Screen - Jetpack Compose
// nexus-android/src/main/java/com/nexus/ui/screen/ChatScreen.kt

package com.nexus.ui.screen

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.input.TextFieldValue
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.nexus.data.model.Message

@Composable
fun ChatScreen(
    conversationId: String,
    recipientName: String,
    messages: List<Message>,
    onSendMessage: (String) -> Unit,
    onCallClick: () -> Unit,
    onVideoCallClick: () -> Unit,
    modifier: Modifier = Modifier
) {
    var messageText by remember { mutableStateOf(TextFieldValue("")) }
    
    Column(
        modifier = modifier
            .fillMaxSize()
            .background(MaterialTheme.colorScheme.background)
    ) {
        // Header
        ChatHeader(
            recipientName = recipientName,
            onCallClick = onCallClick,
            onVideoCallClick = onVideoCallClick
        )
        
        Divider()
        
        // Messages List
        LazyColumn(
            modifier = Modifier
                .weight(1f)
                .fillMaxWidth()
                .padding(8.dp),
            reverseLayout = true
        ) {
            items(messages.reversed()) { message ->
                ChatMessageBubble(
                    message = message,
                    isOwn = message.senderId == "currentUser"
                )
            }
        }
        
        Divider()
        
        // Input Area
        ChatInputArea(
            messageText = messageText,
            onMessageTextChange = { messageText = it },
            onSendClick = {
                if (messageText.text.isNotBlank()) {
                    onSendMessage(messageText.text)
                    messageText = TextFieldValue("")
                }
            }
        )
    }
}

@Composable
private fun ChatHeader(
    recipientName: String,
    onCallClick: () -> Unit,
    onVideoCallClick: () -> Unit
) {
    TopAppBar(
        title = { Text(recipientName) },
        actions = {
            IconButton(onClick = onCallClick) {
                Icon(Icons.Default.Phone, contentDescription = "Voice call")
            }
            IconButton(onClick = onVideoCallClick) {
                Icon(Icons.Default.Videocam, contentDescription = "Video call")
            }
            IconButton(onClick = { /* Menu */ }) {
                Icon(Icons.Default.MoreVert, contentDescription = "More")
            }
        },
        colors = TopAppBarDefaults.topAppBarColors(
            containerColor = MaterialTheme.colorScheme.primary
        )
    )
}

@Composable
private fun ChatMessageBubble(
    message: Message,
    isOwn: Boolean,
    modifier: Modifier = Modifier
) {
    Row(
        modifier = modifier
            .fillMaxWidth()
            .padding(vertical = 4.dp),
        horizontalArrangement = if (isOwn) Arrangement.End else Arrangement.Start
    ) {
        Surface(
            modifier = Modifier
                .widthIn(max = 280.dp)
                .wrapContentWidth(),
            shape = RoundedCornerShape(
                topStart = 12.dp,
                topEnd = 12.dp,
                bottomStart = if (isOwn) 12.dp else 0.dp,
                bottomEnd = if (isOwn) 0.dp else 12.dp
            ),
            color = if (isOwn)
                MaterialTheme.colorScheme.primary
            else
                MaterialTheme.colorScheme.surfaceVariant
        ) {
            Column(
                modifier = Modifier.padding(8.dp)
            ) {
                Text(
                    text = message.content,
                    color = if (isOwn)
                        MaterialTheme.colorScheme.onPrimary
                    else
                        MaterialTheme.colorScheme.onSurfaceVariant,
                    fontSize = 14.sp
                )
                Text(
                    text = message.timestamp,
                    fontSize = 10.sp,
                    color = if (isOwn)
                        MaterialTheme.colorScheme.onPrimary.copy(alpha = 0.7f)
                    else
                        MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.7f),
                    modifier = Modifier.align(Alignment.End).padding(top = 4.dp)
                )
            }
        }
    }
}

@Composable
private fun ChatInputArea(
    messageText: TextFieldValue,
    onMessageTextChange: (TextFieldValue) -> Unit,
    onSendClick: () -> Unit,
    modifier: Modifier = Modifier
) {
    Row(
        modifier = modifier
            .fillMaxWidth()
            .padding(8.dp),
        verticalAlignment = Alignment.CenterVertically
    ) {
        IconButton(onClick = { /* Attachment */ }) {
            Icon(Icons.Default.AttachFile, contentDescription = "Attach file")
        }
        
        OutlinedTextField(
            value = messageText,
            onValueChange = onMessageTextChange,
            modifier = Modifier
                .weight(1f)
                .height(40.dp),
            placeholder = { Text("Message...") },
            singleLine = true,
            shape = RoundedCornerShape(24.dp)
        )
        
        IconButton(onClick = onSendClick) {
            Icon(Icons.Default.Send, contentDescription = "Send message")
        }
    }
}

@Composable
fun GroupChatScreen(
    groupId: String,
    groupName: String,
    members: List<String>,
    messages: List<Message>,
    onSendMessage: (String) -> Unit,
    modifier: Modifier = Modifier
) {
    var messageText by remember { mutableStateOf(TextFieldValue("")) }
    
    Column(
        modifier = modifier
            .fillMaxSize()
            .background(MaterialTheme.colorScheme.background)
    ) {
        // Group Header
        TopAppBar(
            title = { 
                Column {
                    Text(groupName)
                    Text("${members.size} members", fontSize = 12.sp)
                }
            },
            actions = {
                IconButton(onClick = { /* Group info */ }) {
                    Icon(Icons.Default.MoreVert, contentDescription = "Group menu")
                }
            },
            colors = TopAppBarDefaults.topAppBarColors(
                containerColor = MaterialTheme.colorScheme.primary
            )
        )
        
        Divider()
        
        // Messages
        LazyColumn(
            modifier = Modifier
                .weight(1f)
                .fillMaxWidth()
                .padding(8.dp),
            reverseLayout = true
        ) {
            items(messages.reversed()) { message ->
                GroupMessageBubble(message)
            }
        }
        
        Divider()
        
        // Input
        ChatInputArea(
            messageText = messageText,
            onMessageTextChange = { messageText = it },
            onSendClick = {
                if (messageText.text.isNotBlank()) {
                    onSendMessage(messageText.text)
                    messageText = TextFieldValue("")
                }
            }
        )
    }
}

@Composable
private fun GroupMessageBubble(
    message: Message,
    modifier: Modifier = Modifier
) {
    Column(
        modifier = modifier
            .fillMaxWidth()
            .padding(vertical = 4.dp)
            .padding(horizontal = 8.dp)
    ) {
        Text(
            text = message.senderName,
            fontSize = 10.sp,
            color = MaterialTheme.colorScheme.primary
        )
        Surface(
            modifier = Modifier
                .widthIn(max = 280.dp)
                .padding(top = 2.dp),
            shape = RoundedCornerShape(12.dp),
            color = MaterialTheme.colorScheme.surfaceVariant
        ) {
            Column(
                modifier = Modifier.padding(8.dp)
            ) {
                Text(
                    text = message.content,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    fontSize = 14.sp
                )
                Text(
                    text = message.timestamp,
                    fontSize = 10.sp,
                    color = MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.7f),
                    modifier = Modifier
                        .align(Alignment.End)
                        .padding(top = 4.dp)
                )
            }
        }
    }
}
