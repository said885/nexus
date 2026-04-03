// Android Settings Screen - Jetpack Compose
// nexus-android/src/main/java/com/nexus/ui/screen/SettingsScreen.kt

package com.nexus.ui.screen

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp

@Composable
fun SettingsScreen(
    onBackClick: () -> Unit,
    modifier: Modifier = Modifier
) {
    var showAbout by remember { mutableStateOf(false) }
    var notificationsEnabled by remember { mutableStateOf(true) }
    var darkModeEnabled by remember { mutableStateOf(true) }
    var endToEndEncrypted by remember { mutableStateOf(true) }
    
    Column(
        modifier = modifier
            .fillMaxSize()
            .background(MaterialTheme.colorScheme.background)
    ) {
        // Header
        TopAppBar(
            title = { Text("Settings") },
            navigationIcon = {
                IconButton(onClick = onBackClick) {
                    Icon(Icons.Default.ArrowBack, contentDescription = "Back")
                }
            },
            colors = TopAppBarDefaults.topAppBarColors(
                containerColor = MaterialTheme.colorScheme.primary
            )
        )
        
        LazyColumn(
            modifier = Modifier
                .fillMaxSize()
                .padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(8.dp)
        ) {
            // Account Section
            item {
                SettingsSectionHeader("Account")
            }
            item {
                SettingItem(
                    icon = Icons.Default.AccountCircle,
                    title = "Profile",
                    subtitle = "Manage your profile information",
                    onClick = { /* Navigate to profile */ }
                )
            }
            item {
                SettingItem(
                    icon = Icons.Default.Lock,
                    title = "Password",
                    subtitle = "Change your password",
                    onClick = { /* Navigate to change password */ }
                )
            }
            item {
                SettingItem(
                    icon = Icons.Default.VerifiedUser,
                    title = "Two-Factor Authentication",
                    subtitle = "Enable 2FA for better security",
                    onClick = { /* Navigate to 2FA */ }
                )
            }
            
            // Privacy Section
            item {
                SettingsSectionHeader("Privacy & Security")
            }
            item {
                SettingToggleItem(
                    icon = Icons.Default.Lock,
                    title = "End-to-End Encryption",
                    subtitle = "Encrypt all your messages",
                    isEnabled = endToEndEncrypted,
                    onToggle = { endToEndEncrypted = it }
                )
            }
            item {
                SettingItem(
                    icon = Icons.Default.LockClock,
                    title = "Message Privacy",
                    subtitle = "Auto-delete messages after time",
                    onClick = { /* Navigate to message privacy */ }
                )
            }
            item {
                SettingItem(
                    icon = Icons.Default.Visibility,
                    title = "Visibility",
                    subtitle = "Control who can see your status",
                    onClick = { /* Navigate to visibility */ }
                )
            }
            
            // Notifications Section
            item {
                SettingsSectionHeader("Notifications")
            }
            item {
                SettingToggleItem(
                    icon = Icons.Default.Notifications,
                    title = "Notifications",
                    subtitle = "Receive push notifications",
                    isEnabled = notificationsEnabled,
                    onToggle = { notificationsEnabled = it }
                )
            }
            item {
                SettingItem(
                    icon = Icons.Default.VolumeUp,
                    title = "Sound",
                    subtitle = "Notification sound settings",
                    onClick = { /* Navigate to sound settings */ }
                )
            }
            
            // Appearance Section
            item {
                SettingsSectionHeader("Appearance")
            }
            item {
                SettingToggleItem(
                    icon = Icons.Default.DarkMode,
                    title = "Dark Mode",
                    subtitle = "Use dark theme",
                    isEnabled = darkModeEnabled,
                    onToggle = { darkModeEnabled = it }
                )
            }
            
            // Storage Section
            item {
                SettingsSectionHeader("Storage & Data")
            }
            item {
                SettingItem(
                    icon = Icons.Default.Storage,
                    title = "Storage Usage",
                    subtitle = "2.3 GB of 100 GB",
                    onClick = { /* Navigate to storage */ }
                )
            }
            item {
                SettingItem(
                    icon = Icons.Default.Delete,
                    title = "Clear Cache",
                    subtitle = "Delete temporary files",
                    onClick = { /* Clear cache */ }
                )
            }
            
            // Help Section
            item {
                SettingsSectionHeader("Help & Support")
            }
            item {
                SettingItem(
                    icon = Icons.Default.ContactSupport,
                    title = "Contact Support",
                    subtitle = "Get help from our team",
                    onClick = { /* Contact support */ }
                )
            }
            item {
                SettingItem(
                    icon = Icons.Default.Description,
                    title = "Privacy Policy",
                    subtitle = "Read our privacy policy",
                    onClick = { /* Open privacy policy */ }
                )
            }
            item {
                SettingItem(
                    icon = Icons.Default.Info,
                    title = "About",
                    subtitle = "Version 1.0.0",
                    onClick = { showAbout = true }
                )
            }
        }
    }
}

@Composable
private fun SettingsSectionHeader(
    title: String,
    modifier: Modifier = Modifier
) {
    Text(
        text = title,
        fontSize = 12.sp,
        color = MaterialTheme.colorScheme.primary,
        modifier = modifier
            .fillMaxWidth()
            .padding(bottom = 8.dp)
    )
}

@Composable
private fun SettingItem(
    icon: androidx.compose.material.icons.materialIcon,
    title: String,
    subtitle: String,
    onClick: () -> Unit,
    modifier: Modifier = Modifier
) {
    Surface(
        modifier = modifier
            .fillMaxWidth()
            .clickable(onClick = onClick),
        color = MaterialTheme.colorScheme.surface,
        shape = androidx.compose.foundation.shape.RoundedCornerShape(8.dp)
    ) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(16.dp),
            verticalAlignment = Alignment.CenterVertically
        ) {
            Icon(
                imageVector = icon,
                contentDescription = title,
                modifier = Modifier.size(24.dp),
                tint = MaterialTheme.colorScheme.primary
            )
            
            Spacer(modifier = Modifier.width(16.dp))
            
            Column(
                modifier = Modifier.weight(1f)
            ) {
                Text(
                    text = title,
                    fontSize = 16.sp,
                    color = MaterialTheme.colorScheme.onSurface
                )
                Text(
                    text = subtitle,
                    fontSize = 12.sp,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
            }
            
            Icon(
                Icons.Default.ChevronRight,
                contentDescription = "Navigate",
                modifier = Modifier.size(24.dp),
                tint = MaterialTheme.colorScheme.onSurfaceVariant
            )
        }
    }
}

@Composable
private fun SettingToggleItem(
    icon: androidx.compose.material.icons.materialIcon,
    title: String,
    subtitle: String,
    isEnabled: Boolean,
    onToggle: (Boolean) -> Unit,
    modifier: Modifier = Modifier
) {
    Surface(
        modifier = modifier
            .fillMaxWidth()
            .clickable { onToggle(!isEnabled) },
        color = MaterialTheme.colorScheme.surface,
        shape = androidx.compose.foundation.shape.RoundedCornerShape(8.dp)
    ) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(16.dp),
            verticalAlignment = Alignment.CenterVertically
        ) {
            Icon(
                imageVector = icon,
                contentDescription = title,
                modifier = Modifier.size(24.dp),
                tint = MaterialTheme.colorScheme.primary
            )
            
            Spacer(modifier = Modifier.width(16.dp))
            
            Column(
                modifier = Modifier.weight(1f)
            ) {
                Text(
                    text = title,
                    fontSize = 16.sp,
                    color = MaterialTheme.colorScheme.onSurface
                )
                Text(
                    text = subtitle,
                    fontSize = 12.sp,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
            }
            
            Switch(
                checked = isEnabled,
                onCheckedChange = onToggle
            )
        }
    }
}

@Composable
fun SecuritySettingsScreen(
    onBackClick: () -> Unit,
    modifier: Modifier = Modifier
) {
    var showDeviceList by remember { mutableStateOf(false) }
    var showBackupCodes by remember { mutableStateOf(false) }
    
    Column(
        modifier = modifier
            .fillMaxSize()
            .background(MaterialTheme.colorScheme.background)
    ) {
        TopAppBar(
            title = { Text("Security") },
            navigationIcon = {
                IconButton(onClick = onBackClick) {
                    Icon(Icons.Default.ArrowBack, contentDescription = "Back")
                }
            }
        )
        
        LazyColumn(
            modifier = Modifier
                .fillMaxSize()
                .padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(8.dp)
        ) {
            item { SettingsSectionHeader("Devices") }
            item {
                SettingItem(
                    icon = Icons.Default.Devices,
                    title = "Active Devices",
                    subtitle = "2 devices using your account",
                    onClick = { showDeviceList = true }
                )
            }
            
            item { SettingsSectionHeader("Recovery") }
            item {
                SettingItem(
                    icon = Icons.Default.VpnKey,
                    title = "Backup Codes",
                    subtitle = "8 recovery codes available",
                    onClick = { showBackupCodes = true }
                )
            }
            
            item { SettingsSectionHeader("Sessions") }
            item {
                SettingItem(
                    icon = Icons.Default.Logout,
                    title = "Sign Out All Devices",
                    subtitle = "End all active sessions",
                    onClick = { /* Sign out all */ }
                )
            }
        }
    }
}
