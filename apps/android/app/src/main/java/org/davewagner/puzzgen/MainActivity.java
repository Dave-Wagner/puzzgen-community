package org.davewagner.puzzgen;

import android.app.NativeActivity;
import android.app.AlertDialog;
import android.os.Bundle;



import android.text.InputType;

import android.widget.EditText;
import android.widget.LinearLayout;
import android.widget.TextView;
import android.widget.Toast;
import java.io.File;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;






/** NativeActivity hosts Bevy's native Vulkan renderer. No browser is embedded. */
public final class MainActivity extends NativeActivity {
    // NativeActivity's dlopen does not register JNI methods with this class loader.
    // Load explicitly before any lifecycle callback calls the audio bridge.
    static { System.loadLibrary("puzzgen_native"); }

    private boolean dialogOpen;
    private boolean resumed;
    private boolean audioFocusHeld;
    private android.media.AudioManager audioManager;
    private android.media.AudioFocusRequest audioRequest;
    private android.content.BroadcastReceiver screenReceiver;
    private static native void nativeAudioAllowed(boolean allowed);
    private static native void nativeSafeArea(int left, int top, int right, int bottom);
    private static native void nativeBack();
    private static native void nativeWeatherCity(String city);
    private static native void nativeWeatherPosition(double latitude,double longitude);
    private static native void nativeWeatherLocationStatus(String status);
    private static final int WEATHER_LOCATION_PERMISSION=41;
    private final android.os.Handler locationHandler=new android.os.Handler(android.os.Looper.getMainLooper());
    private android.location.LocationManager weatherLocations;
    private android.location.LocationListener weatherLocationListener;
    private android.os.CancellationSignal weatherLocationCancellation;
    private Runnable weatherLocationTimeout;
    private int locationGeneration;
    private boolean locationPermissionPending;
    private boolean locationFixPending;
    public String weatherLocation(){
        runOnUiThread(()->{
            if(isFinishing() || locationPermissionPending || locationFixPending)return;
            if(checkSelfPermission(android.Manifest.permission.ACCESS_COARSE_LOCATION)!=android.content.pm.PackageManager.PERMISSION_GRANTED){
                locationPermissionPending=true;
                requestPermissions(new String[]{android.Manifest.permission.ACCESS_COARSE_LOCATION},WEATHER_LOCATION_PERMISSION);
            }else beginWeatherLocation();
        });return "";
    }
    public String cancelWeatherLocation(){runOnUiThread(()->cancelWeatherFix(false));return "";}
    private void cancelWeatherFix(boolean notify){
        boolean pending=locationFixPending;
        locationGeneration++;locationFixPending=false;
        if(weatherLocationTimeout!=null)locationHandler.removeCallbacks(weatherLocationTimeout);
        weatherLocationTimeout=null;
        if(weatherLocationCancellation!=null)weatherLocationCancellation.cancel();
        weatherLocationCancellation=null;
        if(weatherLocations!=null && weatherLocationListener!=null){
            try{weatherLocations.removeUpdates(weatherLocationListener);}catch(SecurityException ignored){}
        }
        weatherLocationListener=null;
        if(notify && pending)nativeWeatherLocationStatus("Location lookup paused. Tap Use my location to try again.");
    }
    private void locationUnavailable(String message){
        android.util.Log.i("PuzzgenWeather","location_lookup_failed: "+message);
        cancelWeatherFix(false);nativeWeatherLocationStatus(message);
        Toast.makeText(this,message,Toast.LENGTH_LONG).show();
    }
    private void acceptWeatherLocation(android.location.Location location,int generation){
        if(generation!=locationGeneration || !locationFixPending)return;
        if(location==null){locationUnavailable("Location unavailable. Try again or choose a city.");return;}
        double lat=location.getLatitude(),lon=location.getLongitude();
        if(!Double.isFinite(lat)||!Double.isFinite(lon)||Math.abs(lat)>90||Math.abs(lon)>180){locationUnavailable("Location unavailable. Choose a city instead.");return;}
        cancelWeatherFix(false);
        // Weather needs an area, not precise coordinates. No location is written to logs.
        nativeWeatherPosition(Math.round(lat*100.0)/100.0,Math.round(lon*100.0)/100.0);
        nativeWeatherLocationStatus("Location selected. Checking the weather for your saved area…");
    }
    @SuppressWarnings("deprecation")
    private void beginWeatherLocation(){
        if(isFinishing() || checkSelfPermission(android.Manifest.permission.ACCESS_COARSE_LOCATION)!=android.content.pm.PackageManager.PERMISSION_GRANTED)return;
        cancelWeatherFix(false);
        weatherLocations=(android.location.LocationManager)getSystemService(LOCATION_SERVICE);
        if(weatherLocations==null){locationUnavailable("Location unavailable. Choose a city instead.");return;}
        java.util.List<String> enabled=weatherLocations.getProviders(true);
        java.util.List<String> providers=new java.util.ArrayList<>();
        // Race available coarse providers instead of relying on fused alone.
        if(enabled.contains(android.location.LocationManager.NETWORK_PROVIDER))providers.add(android.location.LocationManager.NETWORK_PROVIDER);
        if(enabled.contains("fused"))providers.add("fused");
        if(providers.isEmpty()){locationUnavailable("Turn on device Location, or choose a city instead.");return;}
        final int generation=locationGeneration;
        locationFixPending=true;
        nativeWeatherLocationStatus("Finding your approximate location…");
        android.util.Log.i("PuzzgenWeather","location_lookup_started providers="+providers.size());
        android.location.Location recent=null;
        for(String provider:providers){
            try{
                android.location.Location cached=weatherLocations.getLastKnownLocation(provider);
                if(cached==null)continue;
                long age=android.os.SystemClock.elapsedRealtimeNanos()-cached.getElapsedRealtimeNanos();
                if(age>=0 && age<=java.util.concurrent.TimeUnit.MINUTES.toNanos(10)
                    && (recent==null || cached.getElapsedRealtimeNanos()>recent.getElapsedRealtimeNanos()))recent=cached;
            }catch(SecurityException|IllegalArgumentException ignored){}
        }
        if(recent!=null){
            android.util.Log.i("PuzzgenWeather","location_fix_received source=recent_device_cache");
            acceptWeatherLocation(recent,generation);return;
        }
        weatherLocationTimeout=()->{if(generation==locationGeneration && locationFixPending)locationUnavailable("Location timed out. Check device Location is on, then try again or choose a city.");};
        locationHandler.postDelayed(weatherLocationTimeout,30000);
        weatherLocationListener=new android.location.LocationListener(){
            @Override public void onLocationChanged(android.location.Location location){
                if(generation!=locationGeneration || !locationFixPending)return;
                android.util.Log.i("PuzzgenWeather","location_fix_received source=device_update");
                acceptWeatherLocation(location,generation);
            }
            @Override public void onStatusChanged(String provider,int status,Bundle extras){}
            @Override public void onProviderEnabled(String provider){}
            @Override public void onProviderDisabled(String provider){}
        };
        int registered=0;
        for(String provider:providers){
            try{
                weatherLocations.requestLocationUpdates(provider,1000L,0f,weatherLocationListener,android.os.Looper.getMainLooper());
                registered++;
            }catch(SecurityException|IllegalArgumentException ignored){}
        }
        if(registered==0)locationUnavailable("Location unavailable. You can still choose a city.");
    }
    @Override public void onRequestPermissionsResult(int requestCode,String[] permissions,int[] results){
        super.onRequestPermissionsResult(requestCode,permissions,results);
        if(requestCode!=WEATHER_LOCATION_PERMISSION)return;
        locationPermissionPending=false;
        if(checkSelfPermission(android.Manifest.permission.ACCESS_COARSE_LOCATION)==android.content.pm.PackageManager.PERMISSION_GRANTED){
            locationHandler.post(this::beginWeatherLocation);
        }else{
            nativeWeatherLocationStatus("Location permission was not granted. You can still choose a city.");
            Toast.makeText(this,"Location is optional. You can still choose a city.",Toast.LENGTH_LONG).show();
        }
    }
    public String weatherDialog() {
        runOnUiThread(()->{
            cancelWeatherFix(false);
            if(dialogOpen || isFinishing())return;
            dialogOpen=true;
            EditText city=new EditText(this);
            city.setHint("City, state or country");
            city.setSingleLine(true);
            city.setFilters(new android.text.InputFilter[]{new android.text.InputFilter.LengthFilter(100)});
            AlertDialog dialog=new AlertDialog.Builder(this).setTitle("Local garden weather")
                .setMessage("Enter a city, for example Dayton, Ohio. This is sent to Open-Meteo for weather. City entry does not use device location.")
                .setView(city).setPositiveButton("Use city",(d,w)->nativeWeatherCity(city.getText().toString()))
                .setNegativeButton("Cancel",null).create();
            dialog.setOnDismissListener(d->dialogOpen=false);dialog.show();
        });return "";
    }
    @Override public void onCreate(Bundle state) {
        if (BuildConfig.DEBUG) {
            try {
                if (getIntent().getBooleanExtra("puzzgen.glesTrace", false)) android.system.Os.setenv("RUST_LOG", "info,wgpu_hal::gles=debug", true);
                if (getIntent().getBooleanExtra("puzzgen.gles30Review", false)) android.system.Os.setenv("WGPU_GLES_MINOR_VERSION", "0", true);
            } catch (android.system.ErrnoException error) { android.util.Log.w("Puzzgen", "GLES diagnostics unavailable"); }
        }
        super.onCreate(state);
        try {
            if(ConnectionStore.read(this).isEmpty() && !new File(getFilesDir(),"provision-key").exists()) GuestAccess.prepare(this);
        } catch(Exception ignored) { /* Preserve an existing unreadable key; never replace it. */ }
        audioManager=(android.media.AudioManager)getSystemService(AUDIO_SERVICE);
        audioRequest=new android.media.AudioFocusRequest.Builder(android.media.AudioManager.AUDIOFOCUS_GAIN)
            .setAudioAttributes(new android.media.AudioAttributes.Builder().setUsage(android.media.AudioAttributes.USAGE_GAME).setContentType(android.media.AudioAttributes.CONTENT_TYPE_MUSIC).build())
            .setOnAudioFocusChangeListener(change -> {
                audioFocusHeld=change==android.media.AudioManager.AUDIOFOCUS_GAIN;
                nativeAudioAllowed(audioFocusHeld && eligibleForAudio());
            }).build();
        screenReceiver=new android.content.BroadcastReceiver(){@Override public void onReceive(android.content.Context context,android.content.Intent intent){updateAudioGate();}};
        android.content.IntentFilter filter=new android.content.IntentFilter();filter.addAction(android.content.Intent.ACTION_SCREEN_OFF);filter.addAction(android.content.Intent.ACTION_SCREEN_ON);filter.addAction(android.content.Intent.ACTION_USER_PRESENT);
        if(android.os.Build.VERSION.SDK_INT>=33)registerReceiver(screenReceiver,filter,android.content.Context.RECEIVER_NOT_EXPORTED);else registerReceiver(screenReceiver,filter);
        nativeAudioAllowed(false);
        if(android.os.Build.VERSION.SDK_INT>=33)getOnBackInvokedDispatcher().registerOnBackInvokedCallback(0,MainActivity::nativeBack);
        getWindow().addFlags(android.view.WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON);
        if (android.os.Build.VERSION.SDK_INT>=30) {
            getWindow().setDecorFitsSystemWindows(false);
            android.view.WindowManager.LayoutParams layout=getWindow().getAttributes();
            layout.layoutInDisplayCutoutMode=android.view.WindowManager.LayoutParams.LAYOUT_IN_DISPLAY_CUTOUT_MODE_ALWAYS;
            getWindow().setAttributes(layout);
            showSystemBars();
        } else {
            // Match setDecorFitsSystemWindows(false) on API 26-29. These are
            // layout flags only: keep the native status/navigation bars visible.
            getWindow().getDecorView().setSystemUiVisibility(
                android.view.View.SYSTEM_UI_FLAG_LAYOUT_STABLE |
                android.view.View.SYSTEM_UI_FLAG_LAYOUT_FULLSCREEN |
                android.view.View.SYSTEM_UI_FLAG_LAYOUT_HIDE_NAVIGATION);
        }
        getWindow().getDecorView().setOnApplyWindowInsetsListener((view,insets)->{
            if(android.os.Build.VERSION.SDK_INT>=30){
                android.graphics.Insets safe=insets.getInsetsIgnoringVisibility(android.view.WindowInsets.Type.systemBars()|android.view.WindowInsets.Type.displayCutout());
                android.graphics.Insets gestures=insets.getInsets(android.view.WindowInsets.Type.mandatorySystemGestures());
                nativeSafeArea(Math.max(safe.left,gestures.left),Math.max(safe.top,gestures.top),Math.max(safe.right,gestures.right),Math.max(safe.bottom,gestures.bottom));
            }else{
                nativeSafeArea(insets.getStableInsetLeft(),insets.getStableInsetTop(),insets.getStableInsetRight(),insets.getStableInsetBottom());
            }
            return insets;
        });
        getWindow().getDecorView().requestApplyInsets();
    }
    private void showSystemBars(){
        if(android.os.Build.VERSION.SDK_INT>=30){
            getWindow().getInsetsController().show(android.view.WindowInsets.Type.systemBars());
            getWindow().getInsetsController().setSystemBarsAppearance(0,android.view.WindowInsetsController.APPEARANCE_LIGHT_STATUS_BARS|android.view.WindowInsetsController.APPEARANCE_LIGHT_NAVIGATION_BARS);
        }
    }
    private boolean eligibleForAudio(){
        // Allows attended ADB layout checks without making any sound.
        if(BuildConfig.DEBUG && getIntent().getBooleanExtra("puzzgen.silentReview",false))return false;
        android.os.PowerManager power=(android.os.PowerManager)getSystemService(POWER_SERVICE);
        android.app.KeyguardManager lock=(android.app.KeyguardManager)getSystemService(KEYGUARD_SERVICE);
        return resumed && hasWindowFocus() && power!=null && power.isInteractive() && lock!=null && !lock.isKeyguardLocked();
    }
    private void updateAudioGate(){
        if(!eligibleForAudio()){
            nativeAudioAllowed(false);
            if(audioFocusHeld && audioManager!=null && audioRequest!=null)audioManager.abandonAudioFocusRequest(audioRequest);
            audioFocusHeld=false;return;
        }
        if(!audioFocusHeld && audioManager!=null && audioRequest!=null)audioFocusHeld=audioManager.requestAudioFocus(audioRequest)==android.media.AudioManager.AUDIOFOCUS_REQUEST_GRANTED;
        nativeAudioAllowed(audioFocusHeld);
    }
    @Override protected void onResume(){super.onResume();resumed=true;showSystemBars();updateAudioGate();}
    @Override protected void onPause(){resumed=false;cancelWeatherFix(true);updateAudioGate();super.onPause();}
    @Override public void onWindowFocusChanged(boolean focused){super.onWindowFocusChanged(focused);if(focused)showSystemBars();updateAudioGate();}
    @Override protected void onDestroy(){resumed=false;cancelWeatherFix(false);updateAudioGate();if(screenReceiver!=null)unregisterReceiver(screenReceiver);super.onDestroy();}
    @Override protected void onNewIntent(android.content.Intent intent){super.onNewIntent(intent);setIntent(intent);updateAudioGate();}
    // Keep the native event loop with its task; a second loop cannot be created in this process.
    @android.annotation.SuppressLint("GestureBackNavigation")
    @SuppressWarnings("deprecation")
    @Override public void onBackPressed(){nativeBack();}
    public String backgroundTask(){runOnUiThread(()->moveTaskToBack(true));return "";}
    public String debugInput(){return BuildConfig.DEBUG ? "true" : "false";}
    public String dataDirectory() { return getFilesDir().getAbsolutePath(); }
    private void saveKey(String key) throws Exception { ConnectionStore.write(this,key); }
    public synchronized String apiKey() {
        try {
            // Debug-only ADB provisioning uses stdin into private app storage, never APK assets.
            if (BuildConfig.DEBUG) {
                File provision=new File(getFilesDir(),"provision-key");
                if (provision.isFile() && provision.length()<4096) {
                    String key=new String(Files.readAllBytes(provision.toPath()),StandardCharsets.UTF_8).trim();
                    saveKey(key);Files.delete(provision.toPath());
                }
            }
            String family=ConnectionStore.read(this);
            if(!family.isEmpty()) return family;
            GuestAccess.prepare(this);
            return GuestAccess.configured() ? "@guest" : "";
        } catch (Exception ignored) { return ""; }
    }
    public String guestToken(){return GuestAccess.token(this);}
    public String apiOrigin(){return BuildConfig.BASE_URL;}
    public String connectionDialog() {
        runOnUiThread(() -> {
            if (dialogOpen || isFinishing()) return;dialogOpen=true;
            if(!BuildConfig.DEBUG) {
                AlertDialog guest=new AlertDialog.Builder(this).setTitle("Online puzzles")
                    .setMessage("Online play connects automatically. No login or key is needed. Your saved puzzles and practice remain available offline.")
                    .setPositiveButton("Done",(d,w)->{}).create();
                guest.setOnDismissListener(d->dialogOpen=false);guest.show();return;
            }
            LinearLayout content=new LinearLayout(this);content.setOrientation(LinearLayout.VERTICAL);content.setPadding(32,20,32,12);
            TextView info=new TextView(this);info.setText("Online play connects automatically as a guest. No login or key is needed. Your saved puzzles and practice remain available offline.\n\nAn existing family development key keeps its current access. A replacement key is optional and stays encrypted on this device.");info.setTextSize(18);content.addView(info);
            EditText key=new EditText(this);key.setHint("Development API key");key.setSingleLine(true);key.setInputType(InputType.TYPE_CLASS_TEXT|InputType.TYPE_TEXT_VARIATION_PASSWORD);key.setImportantForAutofill(android.view.View.IMPORTANT_FOR_AUTOFILL_NO);content.addView(key);
            AlertDialog dialog=new AlertDialog.Builder(this).setTitle("Connect online puzzles").setView(content)
                .setPositiveButton("Save key",(d,w)->{try{saveKey(key.getText().toString());Toast.makeText(this,"Key saved. Tap New puzzle to connect.",Toast.LENGTH_LONG).show();}catch(Exception ignored){Toast.makeText(this,"Could not save the key.",Toast.LENGTH_LONG).show();}key.setText("");})
                .setNeutralButton("Remove key",(d,w)->{getSharedPreferences("native-connection",MODE_PRIVATE).edit().clear().apply();key.setText("");})
                .setNegativeButton("Cancel",(d,w)->key.setText("")).create();
            dialog.setOnDismissListener(d->{key.setText("");dialogOpen=false;});dialog.show();
        });return "";
    }
    public String licenseDialog() {
        runOnUiThread(() -> {
            StringBuilder notices=new StringBuilder("Puzzgen original code and procedural music: MIT\n\n");
            try {
                String[] files=getAssets().list("licenses");
                if (files!=null) for(String name:files) {
                    try(java.io.InputStream input=getAssets().open("licenses/"+name);java.io.ByteArrayOutputStream output=new java.io.ByteArrayOutputStream()) {
                        byte[] buffer=new byte[4096];int count;while((count=input.read(buffer))!=-1) output.write(buffer,0,count);
                        notices.append("\n\n--- ").append(name).append(" ---\n\n").append(output.toString("UTF-8"));
                    }
                }
            } catch(java.io.IOException ignored) {notices.append("Some notices could not be opened.");}
            android.widget.ScrollView scroll=new android.widget.ScrollView(this);TextView text=new TextView(this);text.setPadding(28,24,28,24);text.setTextSize(18);text.setText(notices);scroll.addView(text);
            new AlertDialog.Builder(this).setTitle("Credits and original licenses").setView(scroll).setPositiveButton("Close",null).show();
        });return "";
    }
}
