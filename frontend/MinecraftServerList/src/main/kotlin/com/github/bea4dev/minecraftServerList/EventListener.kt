package com.github.bea4dev.minecraftServerList

import org.bukkit.block.Sign
import org.bukkit.block.sign.Side
import org.bukkit.event.EventHandler
import org.bukkit.event.Listener
import org.bukkit.event.block.Action
import org.bukkit.event.player.PlayerInteractEvent

class EventListener : Listener {
    @EventHandler
    fun onPLayerClickSign(event: PlayerInteractEvent) {
        if (event.action != Action.RIGHT_CLICK_BLOCK) {
            return
        }

        val player = event.player
        val block = event.clickedBlock!!
        val sign = block.state as? Sign ?: return

        if (sign.getSide(Side.FRONT).getLine(2) == "[ Server List ]"
            || sign.getSide(Side.BACK).getLine(2) == "[ Server List ]"
        ) {
            ServerListGUIRegistry.PLAYERS_ORDER.open(player)
        }
    }
}