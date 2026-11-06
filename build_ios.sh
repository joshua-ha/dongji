#!/usr/bin/env bash
# Mac에서 실행: Rust 엔진을 iOS용으로 빌드 → XCFramework → Xcode 프로젝트 생성
set -euo pipefail
cd "$(dirname "$0")"

rustup target add aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios
cargo build --release --lib --target aarch64-apple-ios
cargo build --release --lib --target aarch64-apple-ios-sim
cargo build --release --lib --target x86_64-apple-ios

# 시뮬레이터용은 arm64 + x86_64 를 하나로 합친다
mkdir -p target/ios-sim-universal
lipo -create \
  target/aarch64-apple-ios-sim/release/libdongji.a \
  target/x86_64-apple-ios/release/libdongji.a \
  -output target/ios-sim-universal/libdongji.a

rm -rf Dongji.xcframework
xcodebuild -create-xcframework \
  -library target/aarch64-apple-ios/release/libdongji.a -headers include \
  -library target/ios-sim-universal/libdongji.a -headers include \
  -output Dongji.xcframework

# 에셋 재생성이 필요하면: python3 tools/make_assets.py
cd ios
command -v xcodegen >/dev/null || { echo "xcodegen 필요: brew install xcodegen"; exit 1; }
xcodegen generate
echo "완료: open ios/Dongji.xcodeproj  (Signing에서 팀 선택 후 iPhone/iPad로 실행)"
