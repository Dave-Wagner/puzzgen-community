package org.davewagner.puzzgen;

import android.content.ContentProvider;
import android.content.ContentValues;
import android.content.pm.PackageManager;
import android.database.Cursor;
import android.net.Uri;
import android.os.Binder;
import android.os.ParcelFileDescriptor;
import java.io.File;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;

/** Transitional debug-only bridge. No Activity, native renderer, disk key export or release endpoint. */
public final class LegacyMigrationProvider extends ContentProvider {
    @Override public boolean onCreate(){return true;}
    private void authorize() {
        if(!getContext().getPackageName().equals("org.davewagner.puzzgen.dev")) throw new SecurityException("Not legacy app");
        int uid=Binder.getCallingUid();
        PackageManager pm=getContext().getPackageManager();
        String[] packages=pm.getPackagesForUid(uid);
        boolean allowed=false;
        if(packages!=null) for(String name:packages) if(name.equals("org.davewagner.puzzlegen.beaconandbough.dev")) allowed=true;
        if(!allowed || pm.checkSignatures(uid,android.os.Process.myUid())!=PackageManager.SIGNATURE_MATCH)
            throw new SecurityException("Migration caller denied");
    }
    @Override public ParcelFileDescriptor openFile(Uri uri,String mode) throws java.io.FileNotFoundException {
        authorize();
        if(!mode.equals("r")) throw new SecurityException("Read only");
        try {
            byte[] data;
            if("/save".equals(uri.getPath())) {
                File file=new File(getContext().getFilesDir(),"progress-v2.json");
                if(!file.isFile() || file.length()>2*1024*1024) throw new java.io.IOException("Missing or oversized save");
                data=Files.readAllBytes(file.toPath());
            } else if("/key".equals(uri.getPath())) {
                data=ConnectionStore.read(getContext()).getBytes(StandardCharsets.UTF_8);
                if(data.length>4096) throw new java.io.IOException("Oversized key");
            } else throw new java.io.IOException("Unknown migration resource");
            // ContentProvider owns the pipe writer lifecycle, including a closed reader.
            return openPipeHelper(uri,null,null,data,(output,u,m,o,bytes)-> {
                try(java.io.OutputStream stream=new ParcelFileDescriptor.AutoCloseOutputStream(output)) { stream.write(bytes); }
                catch(java.io.IOException ignored) { }
                finally { java.util.Arrays.fill(bytes,(byte)0); }
            });
        } catch(Exception ignored) { throw new java.io.FileNotFoundException("Migration resource unavailable"); }
    }
    @Override public Cursor query(Uri u,String[] p,String s,String[] a,String o){throw new UnsupportedOperationException();}
    @Override public String getType(Uri u){return "application/octet-stream";}
    @Override public Uri insert(Uri u,ContentValues v){throw new UnsupportedOperationException();}
    @Override public int delete(Uri u,String s,String[] a){throw new UnsupportedOperationException();}
    @Override public int update(Uri u,ContentValues v,String s,String[] a){throw new UnsupportedOperationException();}
}
