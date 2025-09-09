package com.github.bea4dev.minecraftServerList

import com.github.bea4dev.artgui.ArtGUI
import org.bukkit.Bukkit
import org.bukkit.plugin.java.JavaPlugin

class MinecraftServerList : JavaPlugin() {
    companion object {
        lateinit var plugin: MinecraftServerList
            private set
        lateinit var artGUI: ArtGUI
            private set
    }

    override fun onEnable() {
        plugin = this
        artGUI = ArtGUI(this)

        val pluginManager = Bukkit.getPluginManager()
        pluginManager.registerEvents(EventListener(), this)

        PluginConfig.load()
        ServerListGUIRegistry.init()
        ServerListService.init(PluginConfig.url)
    }

    override fun onDisable() {}
}
