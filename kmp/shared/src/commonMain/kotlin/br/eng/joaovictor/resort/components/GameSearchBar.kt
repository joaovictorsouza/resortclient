package br.eng.joaovictor.resort.components

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import br.eng.joaovictor.resort.views.preview.PreviewScene
import org.jetbrains.compose.ui.tooling.preview.Preview

/**
 * Compact single-line search field. It is built on [BasicTextField] on purpose: Material's
 * `OutlinedTextField` has a 56dp minimum height and 16dp vertical content padding, so forcing it to a
 * small height leaves no room for the text. Here the height is whatever [height] says and the text is
 * centered in it.
 */
@Composable
fun GameSearchBar(
    value: String,
    onValueChange: (String) -> Unit,
    placeholderText: String,
    modifier: Modifier = Modifier,
    height: Dp = 36.dp,
) {
    val shape = RoundedCornerShape(10.dp)
    val textStyle = MaterialTheme.typography.bodyMedium
    val textColor = MaterialTheme.colorScheme.onSurface
    BasicTextField(
        value = value,
        onValueChange = onValueChange,
        singleLine = true,
        textStyle = textStyle.copy(color = textColor),
        cursorBrush = SolidColor(MaterialTheme.colorScheme.primary),
        modifier = modifier.height(height),
        decorationBox = { innerTextField ->
            Box(
                Modifier.fillMaxSize()
                    .background(Color.White.copy(alpha = 0.06f), shape)
                    .border(1.dp, Color.White.copy(alpha = 0.25f), shape)
                    .padding(horizontal = 12.dp),
                contentAlignment = Alignment.CenterStart,
            ) {
                if (value.isEmpty()) {
                    Text(placeholderText, style = textStyle, color = textColor.copy(alpha = 0.5f))
                }
                innerTextField()
            }
        },
    )
}

// ---- Previews (IDE only; see ai-docs/process/previews-compose.md) ----

@Preview(name = "Empty (placeholder)", widthDp = 360, heightDp = 120)
@Composable
private fun GameSearchBarEmptyPreview() = PreviewScene {
    Box(Modifier.padding(16.dp)) {
        GameSearchBar(value = "", onValueChange = {}, placeholderText = "Buscar", modifier = Modifier.fillMaxWidth())
    }
}

@Preview(name = "With text", widthDp = 360, heightDp = 120)
@Composable
private fun GameSearchBarTextPreview() = PreviewScene {
    Box(Modifier.padding(16.dp)) {
        GameSearchBar(value = "poltrona", onValueChange = {}, placeholderText = "Buscar", modifier = Modifier.fillMaxWidth())
    }
}
