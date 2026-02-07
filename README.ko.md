# Cuayo widget

Cuayo widget은 Windows용 Tauri 기반 데스크톱 캐릭터 위젯입니다.

## 주요 기능

- 투명한 항상-위(Always on top) 캐릭터 창
- 우클릭 설정 패널 (크기, Pumpkin, Exit)
- 호박 상호작용 (생성, 드래그, 추적, 먹기)
- 배고픔 시스템
  - 시작값 `100`
  - 시간 경과로 감소 (`분당 10`)
  - 호박 섭취 시 `+10` (최대 `100`)
  - 배고픔 구간에 따른 표정/음성 반응
- 배고픔 수치 로컬 스토리지 저장

## 기술 스택

- Frontend: HTML / CSS / Vanilla JavaScript (`web/`)
- Desktop runtime: Tauri v2
- Backend: Rust (`src-tauri/`)

## 사전 준비

- Node.js (LTS 권장)
- Rust + Cargo
- Windows 환경 (번들 타겟: `msi`)
- Microsoft Edge WebView2 Runtime
  - 앱 실행에 WebView2가 필요합니다.

## 개발 실행

```bash
npm install
npm run dev
```

## 빌드

```bash
npm run build
```

빌드 산출물:

- EXE: `src-tauri/target/release/app.exe`
- MSI: `src-tauri/target/release/bundle/msi/Cuayo widget_1.0.0_x64_en-US.msi`

## 프로젝트 구조

```text
cuayo-widget/
|- web/                 # 프론트엔드 정적 파일 (UI, 음성, 이미지)
|- src-tauri/           # Tauri/Rust 앱 코드 및 번들 설정
|- package.json         # npm 스크립트 (dev/build)
`- README.md
```

## 버전

현재 버전: `1.0.0`

