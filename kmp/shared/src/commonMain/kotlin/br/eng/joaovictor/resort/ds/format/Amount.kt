package br.eng.joaovictor.resort.ds.format

import kotlin.math.abs

/** `1234567` -> `1.234.567` (pt-BR thousands separator), like the game's `format_amount`. */
fun formatGroupedAmount(value: Long): String {
    val digits = abs(value).toString()
    val grouped = digits.reversed().chunked(3).joinToString(".").reversed()
    return if (value < 0) "-$grouped" else grouped
}
