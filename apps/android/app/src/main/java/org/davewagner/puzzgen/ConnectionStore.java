package org.davewagner.puzzgen;

import android.content.Context;
import android.content.SharedPreferences;
import android.security.keystore.KeyGenParameterSpec;
import android.security.keystore.KeyProperties;
import android.util.Base64;
import java.nio.charset.StandardCharsets;
import java.security.KeyStore;
import javax.crypto.Cipher;
import javax.crypto.KeyGenerator;
import javax.crypto.SecretKey;
import javax.crypto.spec.GCMParameterSpec;

/** The existing on-device format. Never put a shared development key in an APK. */
final class ConnectionStore {
    private static SecretKey key() throws Exception {
        KeyStore store=KeyStore.getInstance("AndroidKeyStore"); store.load(null);
        String alias="puzzgen-dev-api-v1";
        if(!store.containsAlias(alias)) {
            KeyGenerator generator=KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES,"AndroidKeyStore");
            generator.init(new KeyGenParameterSpec.Builder(alias,KeyProperties.PURPOSE_ENCRYPT|KeyProperties.PURPOSE_DECRYPT)
                .setBlockModes(KeyProperties.BLOCK_MODE_GCM).setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE).build());
            generator.generateKey();
        }
        return (SecretKey)store.getKey(alias,null);
    }
    static void write(Context context,String value) throws Exception {
        Cipher cipher=Cipher.getInstance("AES/GCM/NoPadding"); cipher.init(Cipher.ENCRYPT_MODE,key());
        byte[] encrypted=cipher.doFinal(value.trim().getBytes(StandardCharsets.UTF_8));
        if(!context.getSharedPreferences("native-connection",Context.MODE_PRIVATE).edit()
            .putString("iv",Base64.encodeToString(cipher.getIV(),Base64.NO_WRAP))
            .putString("key",Base64.encodeToString(encrypted,Base64.NO_WRAP)).commit())
            throw new java.io.IOException("Could not persist connection");
    }
    static String read(Context context) throws Exception {
        SharedPreferences prefs=context.getSharedPreferences("native-connection",Context.MODE_PRIVATE);
        String encrypted=prefs.getString("key",""); if(encrypted.isEmpty()) return "";
        Cipher cipher=Cipher.getInstance("AES/GCM/NoPadding");
        cipher.init(Cipher.DECRYPT_MODE,key(),new GCMParameterSpec(128,Base64.decode(prefs.getString("iv",""),Base64.NO_WRAP)));
        return new String(cipher.doFinal(Base64.decode(encrypted,Base64.NO_WRAP)),StandardCharsets.UTF_8);
    }
}
