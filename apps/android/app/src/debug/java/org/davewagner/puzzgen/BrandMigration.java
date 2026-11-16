package org.davewagner.puzzgen;

import android.app.Instrumentation;
import android.content.Context;
import android.net.Uri;
import android.os.Bundle;
import android.util.AtomicFile;
import java.io.File;
import java.io.FileOutputStream;
import java.io.InputStream;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.security.MessageDigest;
import java.util.Arrays;

/** Explicit UI-less ADB migration. Source app is untouched; refuses to overwrite a different save. */
public final class BrandMigration extends Instrumentation {
    private boolean fixture;
    @Override public void onCreate(Bundle arguments){super.onCreate(arguments);fixture=arguments!=null && "true".equals(arguments.getString("emulator_fixture"));start();}
    private byte[] read(String path,int limit) throws Exception {
        try(InputStream in=getTargetContext().getContentResolver().openInputStream(
            Uri.parse("content://org.davewagner.puzzgen.dev.migration/"+path));
            java.io.ByteArrayOutputStream out=new java.io.ByteArrayOutputStream()) {
            if(in==null) throw new java.io.IOException();
            byte[] buffer=new byte[4096]; int n;
            while((n=in.read(buffer))!=-1){ if(out.size()+n>limit) throw new java.io.IOException(); out.write(buffer,0,n); }
            return out.toByteArray();
        }
    }
    @Override public void onStart() {
        Bundle result=new Bundle(); byte[] keyBytes=null;
        try {
            Context context=getTargetContext();
            if(fixture) {
                if(!context.getPackageName().equals("org.davewagner.puzzgen.dev") ||
                    !(android.os.Build.HARDWARE.equals("ranchu") || android.os.Build.HARDWARE.equals("goldfish"))) throw new SecurityException();
                File target=new File(context.getFilesDir(),"progress-v2.json");
                if(target.exists()) throw new IllegalStateException();
                Files.write(target.toPath(),"{\"migration_fixture\":true,\"garden\":{\"care\":37}}".getBytes(StandardCharsets.UTF_8));
                ConnectionStore.write(context,"emulator-migration-fixture-not-a-live-key");
                result.putString("fixture","created");finish(-1,result);return;
            }
            if(!context.getPackageName().equals("org.davewagner.puzzlegen.beaconandbough.dev")) throw new SecurityException();
            byte[] save=read("save",2*1024*1024);
            new org.json.JSONObject(new String(save,StandardCharsets.UTF_8));
            keyBytes=read("key",4096); String key=new String(keyBytes,StandardCharsets.UTF_8);
            File target=new File(context.getFilesDir(),"progress-v2.json");
            if(target.exists() && !Arrays.equals(Files.readAllBytes(target.toPath()),save)) throw new IllegalStateException();
            String previous=ConnectionStore.read(context);
            if(!previous.isEmpty() && !previous.equals(key)) throw new IllegalStateException();
            ConnectionStore.write(context,key);
            if(!ConnectionStore.read(context).equals(key)) throw new IllegalStateException();
            AtomicFile atomic=new AtomicFile(target); FileOutputStream output=atomic.startWrite();
            try {output.write(save);atomic.finishWrite(output);} catch(Exception e){atomic.failWrite(output);throw e;}
            if(!Arrays.equals(Files.readAllBytes(target.toPath()),save)) throw new IllegalStateException();
            StringBuilder hash=new StringBuilder();
            for(byte b:MessageDigest.getInstance("SHA-256").digest(save)) hash.append(String.format("%02x",b & 255));
            result.putString("save_sha256",hash.toString());
            result.putBoolean("key_preserved",true);
            result.putBoolean("key_present",!key.isEmpty());
            result.putString("migration","passed");finish(-1,result);
        } catch(Exception e) {
            // No exceptions containing private save data, locations or credentials in output.
            result.putString("migration","failed; source untouched");
            result.putString("failure_type",e.getClass().getSimpleName());finish(0,result);
        } finally {if(keyBytes!=null) Arrays.fill(keyBytes,(byte)0);}
    }
}
