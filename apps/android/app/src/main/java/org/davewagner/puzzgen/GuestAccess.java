package org.davewagner.puzzgen;

import android.content.Context;
import com.google.android.gms.tasks.Tasks;
import com.google.firebase.FirebaseApp;
import com.google.firebase.FirebaseOptions;
import com.google.firebase.auth.FirebaseAuth;
import com.google.firebase.auth.FirebaseUser;
import java.util.concurrent.TimeUnit;

/** Nonblocking warmup; token acquisition waits only on a native HTTP worker. */
final class GuestAccess {
    private static volatile FirebaseAuth auth;
    static volatile com.google.android.gms.tasks.Task<com.google.firebase.auth.AuthResult> pending;
    private static volatile long retryAfter;
    static volatile String lastFailure="none";
    static volatile String phase="idle";
    static boolean configured(){return !BuildConfig.FIREBASE_PROJECT_ID.isEmpty() && !BuildConfig.FIREBASE_APP_ID.isEmpty() && !BuildConfig.FIREBASE_API_KEY.isEmpty();}
    static synchronized void prepare(Context context) {
        if(!configured() || android.os.SystemClock.elapsedRealtime()<retryAfter) return;
        try {
            initialize(context);
            if(auth.getCurrentUser()==null && (pending==null || pending.isComplete())) pending=auth.signInAnonymously();
        } catch(Exception unavailable){retryAfter=android.os.SystemClock.elapsedRealtime()+30000;}
    }
    private static void initialize(Context context) {
        if(auth!=null) return;
        FirebaseOptions options=new FirebaseOptions.Builder().setProjectId(BuildConfig.FIREBASE_PROJECT_ID)
            .setApplicationId(BuildConfig.FIREBASE_APP_ID).setApiKey(BuildConfig.FIREBASE_API_KEY).build();
        FirebaseApp app;
        try {app=FirebaseApp.getInstance("beacon-guest");}
        catch(IllegalStateException absent){app=FirebaseApp.initializeApp(context.getApplicationContext(),options,"beacon-guest");}
        auth=FirebaseAuth.getInstance(app);
    }
    static String token(Context context) {
        if(!configured()){lastFailure="configuration_missing";return "";}
        if(android.os.Looper.myLooper()==android.os.Looper.getMainLooper()){lastFailure="main_thread_denied";return "";}
        // A delayed successful signup is usable immediately, even after this caller timed out.
        if(android.os.SystemClock.elapsedRealtime()<retryAfter && (pending==null || !pending.isSuccessful())) return "";
        try {
            long deadline=android.os.SystemClock.elapsedRealtime()+8000;
            FirebaseUser user;
            com.google.android.gms.tasks.Task<com.google.firebase.auth.AuthResult> signIn;
            phase="sign_in";
            synchronized(GuestAccess.class) {
                initialize(context);
                user=auth.getCurrentUser();
                if(user==null && (pending==null || pending.isComplete())) pending=auth.signInAnonymously();
                signIn=pending;
            }
            // Never hold the initialization lock while waiting: prepare() may run on UI/render.
            if(user==null) user=Tasks.await(signIn,Math.max(1,deadline-android.os.SystemClock.elapsedRealtime()),TimeUnit.MILLISECONDS).getUser();
            if(user==null || !user.isAnonymous()) return "";
            phase="id_token";
            String token=Tasks.await(user.getIdToken(false),Math.max(1,deadline-android.os.SystemClock.elapsedRealtime()),TimeUnit.MILLISECONDS).getToken();
            if(token==null || token.isEmpty()) throw new IllegalStateException();
            synchronized(GuestAccess.class){if(pending==signIn)pending=null;}
            retryAfter=0;lastFailure="none";phase="complete";
            return token;
        } catch(Exception unavailable) {
            // Never log ID/refresh tokens, user IDs or response bodies; no retry loop.
            retryAfter=android.os.SystemClock.elapsedRealtime()+30000;
            Throwable reason=unavailable.getCause()==null ? unavailable : unavailable.getCause();
            lastFailure=reason instanceof com.google.firebase.auth.FirebaseAuthException ? ((com.google.firebase.auth.FirebaseAuthException)reason).getErrorCode() : reason.getClass().getSimpleName();
            return "";
        }
    }
}
