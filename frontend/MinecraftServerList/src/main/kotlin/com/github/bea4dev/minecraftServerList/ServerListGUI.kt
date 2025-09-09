package com.github.bea4dev.minecraftServerList

import com.github.bea4dev.artgui.button.*
import com.github.bea4dev.artgui.frame.Artist
import com.github.bea4dev.artgui.menu.ArtMenu
import org.bukkit.Material
import org.bukkit.Sound
import org.bukkit.entity.Player
import org.lang.tyml.Ordering
import kotlin.math.max

object ServerListGUIRegistry {
    val PLAYERS_ORDER = ServerListGUI(Ordering.PLAYER)
    val PLAYERS_REVERSE_ORDER = ServerListGUI(Ordering.PLAYERREVERSE)

    fun init() {
        ServerListService.onUpdate {
            PLAYERS_ORDER.initGUI()
            PLAYERS_REVERSE_ORDER.initGUI()
        }
    }
}

class ServerListGUI(private val ordering: Ordering) {
    private var artMenu: ArtMenu

    init {
        artMenu = initGUI()
    }

    fun initGUI(): ArtMenu {
        // Artistクラスのインスタンスを作成
        // GUIの大きさと全てのページに配置するボタンを定義する
        val artist = Artist({
            // nullを指定すると空白になりアイテムを配置したりできるようになる
            val V: ArtButton? = null
            // ボタンを作成
            val G = ArtButton(ItemBuilder(Material.GRAY_STAINED_GLASS_PANE).name("&a").build())

            // ページ移動用ボタンを作成
            val N =
                PageNextButton(ItemBuilder(Material.ARROW).name("&r次のページ | Next &7[{NextPage}/{MaxPage}]").build())

            //ページ移動用ボタンを作成
            val P =
                PageBackButton(
                    ItemBuilder(Material.ARROW).name("&r前のページ | Prev &7[{PreviousPage}/{MaxPage}]").build()
                )
            // 閉じるボタンを作成
            val C = ArtButton(
                ItemBuilder(Material.OAK_DOOR).name("&r&6閉じる").build()
            ).listener { event, _ -> event.whoClicked.closeInventory() }

            // 現在のページを表示するボタンを作成
            val I = ReplaceableButton(
                ItemBuilder(Material.NAME_TAG).name("&7現在のページ | Current &r[{CurrentPage}/{MaxPage}]").build()
            )

            // サーバー順序切り替え
            val R = when (ordering) {
                Ordering.PLAYER -> ArtButton(
                    ItemBuilder(Material.LEVER).name("&r人数の少ない順 | Reverse order").build()
                ).listener { event, _ ->
                    val player = event.whoClicked as? Player ?: return@listener
                    player.playSound(player.location, Sound.UI_BUTTON_CLICK, 1.0F, 2.0F)
                    player.closeInventory()
                    ServerListGUIRegistry.PLAYERS_REVERSE_ORDER.open(player)
                }

                Ordering.PLAYERREVERSE -> ArtButton(
                    ItemBuilder(Material.LEVER).name("&r人数の多い順 | Players count order").build()
                ).listener { event, _ ->
                    val player = event.whoClicked as? Player ?: return@listener
                    player.playSound(player.location, Sound.UI_BUTTON_CLICK, 1.0F, 2.0F)
                    player.closeInventory()
                    ServerListGUIRegistry.PLAYERS_ORDER.open(player)
                }
            }

            arrayOf(
                V, V, V, V, V, V, V, G, N,
                V, V, V, V, V, V, V, G, I,
                V, V, V, V, V, V, V, G, P,
                V, V, V, V, V, V, V, G, G,
                V, V, V, V, V, V, V, G, R,
                V, V, V, V, V, V, V, G, C,
            )
        })

        // GUIを作成
        val artMenu = artist.createMenu(MinecraftServerList.artGUI, "&n外部サーバーリスト&r [{CurrentPage}/{MaxPage}]")

        // 非同期でアイテムを配置
        artMenu.asyncCreate { menu ->
            val serverList = when (ordering) {
                Ordering.PLAYER -> ServerListService.serverListPlayersOrder
                Ordering.PLAYERREVERSE -> ServerListService.serverListPlayersReverseOrder
            }

            for (server in serverList) {
                val icon = try {
                    Material.valueOf(server.icon.uppercase())
                } catch (_: Exception) {
                    Material.GRASS_BLOCK
                }

                val name = "&r${server.name}&r [${server.playersOnline}/${server.playersMax}]"

                val description = mutableListOf("&r&7Version: ${server.versionName}", "")
                description.addAll(server.description.split("\n").map { line -> "&r$line" })

                menu.addButton(
                    ArtButton(
                        ItemBuilder(icon)
                            .name(name)
                            .lore(*description.toTypedArray())
                            .build()
                            .also { item -> item.amount = max(1, server.playersOnline.toInt()) }
                    ).listener { event, _ ->
                        val player = event.whoClicked as? Player ?: return@listener

                        // クリック時にTransferパケットで別サーバーに転送する
                        player.transfer(server.ip, server.port.toInt())
                    }
                )
            }
        }

        return artMenu
    }

    fun open(player: Player) {
        artMenu.open(player)
    }
}