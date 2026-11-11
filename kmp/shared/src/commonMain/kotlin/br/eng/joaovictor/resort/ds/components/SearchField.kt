package br.eng.joaovictor.resort.ds.components

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.Icon
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.platform.LocalSoftwareKeyboardController
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import br.eng.joaovictor.resort.ds.DsPreviewScene
import br.eng.joaovictor.resort.ds.icons.GameIcons
import br.eng.joaovictor.resort.ds.theme.Radius
import br.eng.joaovictor.resort.ds.theme.ResortTheme
import br.eng.joaovictor.resort.ds.theme.Space
import br.eng.joaovictor.resort.ds.theme.Size as DsSize

/**
 * 48dp search field with a magnifier, a clear button while there is text and `ImeAction.Search`. Built on
 * `BasicTextField` because Material's `OutlinedTextField` has a 56dp minimum height and fixed padding.
 */
@Composable
fun SearchField(
    value: String,
    onValueChange: (String) -> Unit,
    placeholder: String,
    modifier: Modifier = Modifier,
    autoFocus: Boolean = false,
    onClear: () -> Unit = { onValueChange("") },
    onSubmit: (() -> Unit)? = null,
) {
    val colors = ResortTheme.colors
    val keyboard = LocalSoftwareKeyboardController.current
    val focus = remember { FocusRequester() }
    LaunchedEffect(autoFocus) {
        if (autoFocus) {
            focus.requestFocus()
            keyboard?.show()
        }
    }
    BasicTextField(
        value = value,
        onValueChange = onValueChange,
        singleLine = true,
        textStyle = ResortTheme.type.body.copy(color = colors.text),
        cursorBrush = SolidColor(colors.secondary),
        keyboardOptions = KeyboardOptions(imeAction = ImeAction.Search),
        keyboardActions = KeyboardActions(onSearch = { keyboard?.hide(); onSubmit?.invoke() }),
        modifier = modifier.height(DsSize.searchHeight).focusRequester(focus),
        decorationBox = { inner ->
            Row(
                Modifier.fillMaxWidth().height(DsSize.searchHeight)
                    .background(colors.surfaceSunken, Radius.pill)
                    .border(1.dp, colors.outline, Radius.pill)
                    .padding(horizontal = Space.md),
                horizontalArrangement = Arrangement.spacedBy(Space.sm),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Icon(GameIcons.Search, null, tint = colors.textMuted, modifier = Modifier.size(DsSize.iconSmall))
                Box(Modifier.weight(1f), contentAlignment = Alignment.CenterStart) {
                    if (value.isEmpty()) Text(placeholder, style = ResortTheme.type.body, color = colors.textMuted, maxLines = 1)
                    inner()
                }
                if (value.isNotEmpty()) {
                    GameIconButton(GameIcons.Close, "Limpar busca", onClear, modifier = Modifier.size(DsSize.touchMin))
                }
            }
        },
    )
}

// ---- Previews ----

@Preview(name = "SearchField", widthDp = 360, heightDp = 180)
@Composable
private fun SearchFieldPreview() = DsPreviewScene {
    androidx.compose.foundation.layout.Column(Modifier.padding(Space.lg), verticalArrangement = Arrangement.spacedBy(Space.md)) {
        SearchField("", {}, "Buscar", Modifier.fillMaxWidth())
        SearchField("poltrona", {}, "Buscar", Modifier.fillMaxWidth())
    }
}
