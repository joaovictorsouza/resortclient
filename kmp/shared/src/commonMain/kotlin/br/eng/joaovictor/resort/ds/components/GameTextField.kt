package br.eng.joaovictor.resort.ds.components

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import br.eng.joaovictor.resort.ds.DsPreviewScene
import br.eng.joaovictor.resort.ds.theme.Radius
import br.eng.joaovictor.resort.ds.theme.ResortTheme
import br.eng.joaovictor.resort.ds.theme.Space
import br.eng.joaovictor.resort.ds.theme.Size as DsSize

/** `12/50` for the character counter. */
fun counterText(length: Int, max: Int): String = "$length/$max"

/** A labelled single-line text field with an optional counter and error message. */
@Composable
fun GameTextField(
    value: String,
    onValueChange: (String) -> Unit,
    label: String,
    modifier: Modifier = Modifier,
    placeholder: String = "",
    maxLength: Int? = null,
    error: String? = null,
    imeAction: ImeAction = ImeAction.Done,
) {
    val colors = ResortTheme.colors
    val frame = if (error != null) colors.dangerText else colors.outline
    Column(modifier, verticalArrangement = Arrangement.spacedBy(Space.xs)) {
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
            Text(label, style = ResortTheme.type.caption, color = colors.textMuted)
            if (maxLength != null) Text(counterText(value.length, maxLength), style = ResortTheme.type.caption, color = colors.textMuted)
        }
        BasicTextField(
            value = value,
            onValueChange = onValueChange,
            singleLine = true,
            textStyle = ResortTheme.type.body.copy(color = colors.text),
            cursorBrush = SolidColor(colors.secondary),
            keyboardOptions = KeyboardOptions(imeAction = imeAction),
            modifier = Modifier.fillMaxWidth().height(DsSize.searchHeight),
            decorationBox = { inner ->
                Box(
                    Modifier.fillMaxWidth().height(DsSize.searchHeight)
                        .background(colors.surfaceSunken, androidx.compose.foundation.shape.RoundedCornerShape(Radius.md))
                        .border(1.dp, frame, androidx.compose.foundation.shape.RoundedCornerShape(Radius.md))
                        .padding(horizontal = Space.md),
                    contentAlignment = Alignment.CenterStart,
                ) {
                    if (value.isEmpty()) Text(placeholder, style = ResortTheme.type.body, color = colors.textMuted, maxLines = 1)
                    inner()
                }
            },
        )
        if (error != null) Text(error, style = ResortTheme.type.caption, color = colors.dangerText)
    }
}

// ---- Previews ----

@Preview(name = "GameTextField", widthDp = 360, heightDp = 300)
@Composable
private fun GameTextFieldPreview() = DsPreviewScene {
    Column(Modifier.padding(Space.lg), verticalArrangement = Arrangement.spacedBy(Space.md)) {
        GameTextField("", {}, "Nome da sala", placeholder = "Até 50 letras", maxLength = 50)
        GameTextField("Casa da Praia", {}, "Nome da sala", maxLength = 50)
        GameTextField("", {}, "Nome da sala", maxLength = 50, error = "Dê um nome ao espaço")
    }
}
