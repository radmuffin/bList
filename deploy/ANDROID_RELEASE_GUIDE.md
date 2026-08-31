# 🤖 Google Play Store Release Guide for bList

This guide walks you through building, signing, and submitting the **bList** Android app to the **Google Play Store**.

---

## 📋 Prerequisites

1. **Google Play Developer Account**:
   - Register at [Google Play Console](https://play.google.com/console/signup) ($25 one-time registration fee).
2. **Android Studio**:
   - Install Android Studio Hedgehog (or later) on your machine.
   - Install Android SDK Platform 34 (Android 14) and Android Build Tools.
3. **Node.js 18+ & npm**:
   - All Capacitor dependencies are installed in `package.json`.

---

## 🛠️ Step 1: Prepare & Sync Assets

Ensure the latest web app files and config are synchronized to the Android project:

```bash
# In the repository root:
npm install
npm run cap:sync
```

---

## 🔑 Step 2: Generate a Release Keystore

Generate an upload key to sign your Android App Bundle (`.aab`):

```bash
keytool -genkey -v -keystore blist-release-key.jks \
  -keyalg RSA -keysize 2048 -validity 10000 \
  -alias blist -storetype JKS
```

> ⚠️ **IMPORTANT**: Back up `blist-release-key.jks` and keep your passwords in a password manager. If you lose this key, you cannot update your app on the Play Store without contacting Google Support.

---

## 📦 Step 3: Build Signed Android App Bundle (.aab)

### Option A: Using Android Studio (GUI - Recommended)
1. Open Android Studio.
2. Select **Open Project** and navigate to the `android/` directory inside this repository.
3. Wait for Gradle sync to complete.
4. In the top menu, go to: **Build → Generate Signed Bundle / APK...**
5. Select **Android App Bundle (.aab)** and click **Next**.
6. Provide the path to `blist-release-key.jks`, your keystore password, and key alias (`blist`).
7. Choose the **release** build variant and destination folder.
8. Click **Create**. The `.aab` file will be generated in `android/app/release/app-release.aab`.

### Option B: Using Gradle CLI
Configure signing in `android/app/build.gradle` or run:
```bash
cd android
./gradlew bundleRelease
```

---

## 🚀 Step 4: Google Play Console Setup

### 1. Create Application
- Go to [Google Play Console](https://play.google.com/console).
- Click **Create App**.
- App Name: `bList - Visual Map Bucket List`
- Default language: `English (United States)`
- App or Game: `App`
- Free or Paid: `Free`

### 2. Store Listing Details
- **Short Description** (up to 80 chars):
  `Visual map bucket list & trip planner. Save places directly from any share sheet.`
- **Full Description** (up to 4000 chars):
  `bList is a fast, visual map bucket list and trip planner. Save places instantly from Google Maps, Instagram, Apple Maps, and travel blogs directly into custom trip lists. View saved spots on an interactive map, calculate distances in real-time, get weather forecasts, and pick random places with "Surprise Me".`
- **App Icon**: 512×512 PNG (32-bit color). Use `static/icons/icon-512.png`.
- **Feature Graphic**: 1024×500 PNG or JPEG.
- **Screenshots**: Upload phone screenshots from the `screenshots/` directory.

### 3. Policy & App Content Declarations
Complete the mandatory questionnaires under **App Content**:
- **Privacy Policy**: `https://blist-radmuffin.fly.dev/privacy.html`
- **App Access**: All functionality is available without special access.
- **Ads**: Select *No, my app does not contain ads*.
- **Content Ratings**: Complete the IARC questionnaire (select *Utility/Travel* — rating will be All Ages / Everyone).
- **Target Audience**: 13 and older.
- **Data Safety Form**:
  - Location: Declared as collected (Approximate location & Precise location) for *App Functionality* (not shared with third parties, processed in real-time for distance and proximity sorting).
  - Personal Info: None collected.

---

## 🧪 Step 5: Testing & Release Track

### For Personal Developer Accounts (Created after Nov 13, 2023)
Google requires **20 closed testers who have been opted-in for at least 14 days** before you can apply for Production access:
1. Go to **Testing → Closed testing**.
2. Create a track (e.g. `Closed Alpha / Beta`).
3. Upload `app-release.aab`.
4. Create an email list with 20+ testers (friends, colleagues, or beta testing groups).
5. Share the opt-in link with your testers.
6. Once the 14-day criteria are met, apply for **Production** access directly in the Play Console dashboard.

### For Organization Developer Accounts
You can publish directly to the **Production** track for Google review.

---

## 🔄 Step 6: Subsequent App Updates

When publishing updates:
1. Increment `versionCode` (e.g. `2`) and update `versionName` (e.g. `"1.1.0"`) in `android/app/build.gradle`.
2. Sync changes: `npm run cap:sync`
3. Generate the new `.aab` file.
4. Create a new release in the Play Console track and upload the `.aab`.
