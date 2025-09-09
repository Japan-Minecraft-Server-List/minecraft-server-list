package com.github.bea4dev.minecraftServerList

import org.bukkit.configuration.file.YamlConfiguration
import java.io.File

object PluginConfig {
    private val file = File("plugins/MinecraftServerList/config.yml")
    private lateinit var yml: YamlConfiguration

    lateinit var url: String
        private set

    fun load() {
        if (!file.exists()) {
            file.parentFile.mkdirs()

            // 初期値を設定
            val yml = YamlConfiguration()
            yml.set("url", "http://localhost:3000")
            yml.save(file)
        }

        yml = YamlConfiguration.loadConfiguration(file)

        url = yml.getString("url")
            ?: throw IllegalStateException("No url found in plugins/MinecraftServerList/config.yml!")
    }
}