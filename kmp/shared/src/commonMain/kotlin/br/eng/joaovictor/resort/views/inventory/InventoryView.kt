package br.eng.joaovictor.resort.views.inventory

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawingPadding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp

/**
 * Example modal screen: Repository -> ViewModel -> View. Not wired into the app yet.
 * A scrim covers the screen (tap closes) and the panel takes the right half.
 */
@Composable
fun InventoryView(viewModel: InventoryViewModel) {
    val ui by viewModel.ui.collectAsState()
    if (!ui.open) return
    Box(
        Modifier.fillMaxSize()
            .background(Color.Black.copy(alpha = 0.5f))
            .clickable(interactionSource = remember { MutableInteractionSource() }, indication = null) { viewModel.close() },
    ) {
        Surface(
            modifier = Modifier.align(Alignment.CenterEnd)
                .fillMaxWidth(0.5f)
                .fillMaxHeight()
                // Swallows taps so only the scrim closes the modal.
                .clickable(interactionSource = remember { MutableInteractionSource() }, indication = null) {},
            color = MaterialTheme.colorScheme.surface,
        ) {
            Column(Modifier.safeDrawingPadding().padding(16.dp)) {
                Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
                    Text("Inventário", style = MaterialTheme.typography.titleLarge, modifier = Modifier.weight(1f))
                    TextButton(onClick = viewModel::close) { Text("Fechar") }
                }
                when {
                    ui.loading -> Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) { CircularProgressIndicator() }
                    ui.isEmpty -> Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) { Text("Seu inventário está vazio") }
                    else -> LazyColumn(verticalArrangement = Arrangement.spacedBy(4.dp)) {
                        items(ui.rows, key = { it.catalogId }) { row ->
                            val bg = if (row.selected) MaterialTheme.colorScheme.primaryContainer else Color.Transparent
                            Row(
                                Modifier.fillMaxWidth().background(bg).clickable { viewModel.select(row.catalogId) }.padding(12.dp),
                                verticalAlignment = Alignment.CenterVertically,
                            ) {
                                Text(row.name, Modifier.weight(1f))
                                row.quantity?.let { Text("x$it") }
                            }
                        }
                    }
                }
            }
        }
    }
}
