package com.github.bea4dev.minecraftServerList

import org.lang.tyml.API
import org.lang.tyml.Ordering
import org.lang.tyml.Server
import java.lang.Thread.sleep
import java.time.Duration
import java.util.concurrent.CopyOnWriteArrayList

object ServerListService {
    private lateinit var api: API
    var serverListPlayersOrder = listOf<Server>()
        private set
    var serverListPlayersReverseOrder = listOf<Server>()
        private set
    private val onUpdate = CopyOnWriteArrayList<Runnable>()

    fun init(url: String) {
        api = API(url)

        // 10分ごとにバックエンドサーバーに問い合わせ
        Thread {
            try {
                serverListPlayersOrder = api.getServerList(Ordering.PLAYER)
                serverListPlayersReverseOrder = api.getServerList(Ordering.PLAYERREVERSE)
            } catch (error: Exception) {
                MinecraftServerList.plugin.logger.warning("Failed to get server list!")
                error.printStackTrace()
            }
            for (task in onUpdate) {
                task.run()
            }
            sleep(Duration.ofMinutes(10).toMillis())
        }.start()
    }

    fun onUpdate(task: Runnable) {
        onUpdate.add(task)
    }
}