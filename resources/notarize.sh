#!/usr/bin/env bash

build_app=$1
if [[ -z "$build_app" ]]; then
    echo "You must supply an app to notarize."
    exit 1
fi

build_app_location="target/release/$build_app"


if [[ -z "$APPLE_SIGN_ID" ]]; then
    echo "You must supply an apple sign id for signing via secrets"
    exit 1
fi
if [[ -z "$APPLE_ID" ]]; then
    echo "You must supply an apple id via secrets"
    exit 1
fi

if [[ -z "$APPLE_ID_PASSWORD" ]]; then
    echo "You must supply an apple password via secrets."
    exit 1
fi

echo "Notarization: signing."
# first lets sign the bin
# codesign --force --deep -s $APPLE_SIGN_ID --options runtime --timestamp $build_app

echo "Notarization: zipping."
# then zip, which is needed for notarize
ditto -c -k --rsrc --keepParent "$build_app_location" "$build_app_location.zip"

echo "Notarization: uploading $build_app_location.zip"
# trigger the notarize
xcrun altool --notarize-app -f "$build_app_location.zip" --primary-bundle-id "com.maidsafe.$build_app" -u "$APPLE_ID" -p "$APPLE_ID_PASSWORD" &> tmp

echo 'Notarization: waiting.'
# and wait for complete
uuid=`cat tmp | grep -Eo '\w{8}-(\w{4}-){3}\w{12}$'`
while true; do
    echo "Checking notarization status"

    xcrun altool --notarization-info "$uuid" --username "$APPLE_ID" --password "$APPLE_ID_PASSWORD" &> tmp
    r=`cat tmp`
    echo "$r"
    t=`echo "$r" | grep "success"`
    f=`echo "$r" | grep "invalid"`
    if [[ "$t" != "" ]]; then
        echo "Notarization successful!"
        xcrun stapler staple "$build_app_location"
        xcrun stapler staple "$build_app_location.zip"
        echo "Notarization stapled to bins successfully"
        exit 0;
        break
    fi
    if [[ "$f" != "" ]]; then
        echo "$r"
        exit 1
    fi
    echo "Waiting on notariation... sleep 2m then check again..."
    sleep 120
done
