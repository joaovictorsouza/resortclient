package br.eng.joaovictor.resort

internal expect fun writeFile(dir: String, name: String, bytes: ByteArray)

internal expect fun platformLog(level: String, target: String, text: String)
