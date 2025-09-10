
package org.lang.tyml

import kotlinx.serialization.*
import kotlinx.serialization.descriptors.*
import kotlinx.serialization.encoding.*
import kotlinx.serialization.json.*
import okhttp3.HttpUrl.Companion.toHttpUrl
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody

sealed class Result<out V, out E>
class Ok<out V>(val value: V) : Result<V, Nothing>()
class Err<out E>(val error: E) : Result<Nothing, E>()
/**
 * サーバーリストの順序
 */
@Serializable
enum class Ordering {
    /**
     * プレイヤーの多い順
     */
    @SerialName("Player") PLAYER,
    /**
     * プレイヤーの少ない順
     */
    @SerialName("PlayerReverse") PLAYERREVERSE,
}

/**
 * サーバーリストの要素
 */
@Serializable
data class Server (
    /**
     * プレイヤー人数
     */
    @SerialName("players_online") val playersOnline: Long,
    /**
     * バージョン名
     */
    @SerialName("version_name") val versionName: String,
    /**
     * サーバーの説明欄
     * 改行可
     */
    @SerialName("description") val description: String,
    /**
     * アイコンとなるアイテム名
     */
    @SerialName("icon") val icon: String,
    /**
     * オンラインかどうか
     */
    @SerialName("is_online") val isOnline: Boolean,
    /**
     * MinecraftサーバーのIPアドレス
     */
    @SerialName("ip") val ip: String,
    /**
     * Minecraftサーバーのポート
     */
    @SerialName("port") val port: Long,
    /**
     * サーバーの名前
     */
    @SerialName("name") val name: String,
    /**
     * 最大プレイ人数
     */
    @SerialName("players_max") val playersMax: Long,
)



/**
 */
class API(private val url: String) {
    /**
     * サーバーリストを取得する
     * 配列の順序はorderingに準拠する
     * 定期的に更新するならキャッシュしても問題ない
     */
    fun getServerList(ordering: Ordering): List<Server> {
        val client = OkHttpClient()
        val url = "${this.url}/api/get_server_list".toHttpUrl().newBuilder()
            .addQueryParameter("ordering", Json.encodeToString(ordering))
            .build()
        val reqBody = ""
            .toRequestBody("application/json".toMediaType())
        val request = Request.Builder()
            .url(url)
            .get()
            .build()
        client.newCall(request).execute().use { response ->
            if (!response.isSuccessful) {
                error("HTTP ${response.code}: ${response.body?.string()}")
            }
            return Json.decodeFromString(response.body!!.string())
        }
    }
}
