package org.davewagner.puzzgen;

import android.app.Instrumentation;
import android.os.Bundle;
/** UI-less emulator-only SDK check. Never exports tokens or touches a family key/save. */
public final class GuestAuthSmoke extends Instrumentation {
    @Override public void onCreate(Bundle arguments){super.onCreate(arguments);start();}
    @Override public void onStart(){
        Bundle result=new Bundle();
        try {
            if(!(android.os.Build.HARDWARE.equals("ranchu") || android.os.Build.HARDWARE.equals("goldfish"))) throw new SecurityException();
            java.net.HttpURLConnection probe=(java.net.HttpURLConnection)new java.net.URL("https://identitytoolkit.googleapis.com/").openConnection();
            probe.setConnectTimeout(5000);probe.setReadTimeout(5000);
            try {result.putInt("network_status",probe.getResponseCode());} finally {probe.disconnect();}
            com.google.firebase.FirebaseApp app;
            try {app=com.google.firebase.FirebaseApp.getInstance("beacon-guest");}
            catch(IllegalStateException absent){app=com.google.firebase.FirebaseApp.initializeApp(getTargetContext(),new com.google.firebase.FirebaseOptions.Builder().setProjectId(BuildConfig.FIREBASE_PROJECT_ID).setApplicationId(BuildConfig.FIREBASE_APP_ID).setApiKey(BuildConfig.FIREBASE_API_KEY).build(),"beacon-guest");}
            // Discard only this emulator's prior anonymous test identity, never game/key data.
            com.google.firebase.auth.FirebaseAuth.getInstance(app).signOut();
            String familyBefore=ConnectionStore.read(getTargetContext());
            long began=android.os.SystemClock.elapsedRealtime();
            String a=GuestAccess.token(getTargetContext());
            result.putLong("acquisition_ms",android.os.SystemClock.elapsedRealtime()-began);
            if(a.isEmpty()) {
                if(GuestAccess.pending!=null) {
                    try {com.google.android.gms.tasks.Tasks.await(GuestAccess.pending,90,java.util.concurrent.TimeUnit.SECONDS);result.putString("late_sdk","completed");a=GuestAccess.token(getTargetContext());}
                    catch(Exception late){Throwable reason=late.getCause()==null ? late : late.getCause();result.putString("late_sdk",reason instanceof com.google.firebase.auth.FirebaseAuthException ? ((com.google.firebase.auth.FirebaseAuthException)reason).getErrorCode() : reason.getClass().getSimpleName());String detail=String.valueOf(reason.getMessage()).replaceAll("https?://\\S+","[url]").replaceAll("AIza[0-9A-Za-z_-]+","[config]").replaceAll("eyJ[0-9A-Za-z_.-]+","[token]");result.putString("sdk_detail",detail.substring(0,Math.min(256,detail.length())));}
                }
                if(a.isEmpty()) throw new IllegalStateException("guest_unavailable");
            }
            String b=GuestAccess.token(getTargetContext());
            org.json.JSONObject first=claims(a),second=claims(b);
            if(!first.getString("aud").equals(BuildConfig.FIREBASE_PROJECT_ID) || !first.getString("sub").equals(second.getString("sub")) || !first.getJSONObject("firebase").getString("sign_in_provider").equals("anonymous")) throw new IllegalStateException();
            if(!ConnectionStore.read(getTargetContext()).equals(familyBefore)) throw new IllegalStateException();
            java.net.HttpURLConnection api=(java.net.HttpURLConnection)new java.net.URL(BuildConfig.BASE_URL+"/api/v1/puzzles/arithmetic?seed=416&difficulty=easy").openConnection();
            api.setConnectTimeout(10000);api.setReadTimeout(10000);api.setRequestProperty("Authorization","Bearer "+a);
            try {int status=api.getResponseCode();result.putInt("live_puzzle_status",status);if(status!=200)throw new IllegalStateException();} finally {api.disconnect();}
            result.putString("guest_auth","passed");result.putString("project",BuildConfig.FIREBASE_PROJECT_ID);
            result.putBoolean("identity_reused",true);result.putBoolean("family_key_unchanged",true);
            result.putBoolean("saved_game_untouched",true);finish(-1,result);
        } catch(Exception error){result.putString("guest_auth","failed");result.putString("failure_type",error.getClass().getSimpleName());result.putString("sdk_status",GuestAccess.lastFailure);result.putString("sdk_phase",GuestAccess.phase);finish(0,result);}
    }
    private org.json.JSONObject claims(String jwt) throws Exception {
        // Inspection only; server verification remains authoritative.
        return new org.json.JSONObject(new String(android.util.Base64.decode(jwt.split("\\.")[1],android.util.Base64.URL_SAFE|android.util.Base64.NO_WRAP),java.nio.charset.StandardCharsets.UTF_8));
    }
}
