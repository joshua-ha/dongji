// 冬至夜行 (동지야행) — C ABI 헤더. Swift Bridging Header에서 #include 하세요.
#ifndef DONGJI_H
#define DONGJI_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct Game Game;

// 게임 생성/해제
Game *dongji_new(uint64_t seed);
void dongji_free(Game *game);

// 아래 함수들은 Turn JSON 문자열을 돌려준다. 사용 후 dongji_string_free 필수.
// {"logs":[{"kind":"narration|dialogue|title|system|status|battle|danger|reward",
//           "speaker":string|null,"text":string}...],
//  "prompt":{"type":"choice|text","title":string,"options":[string...]},
//  "checkpoint":string|null}   // null이 아니면 그대로 저장해 두면 된다 (자동 세이브)
char *dongji_start(Game *game);
char *dongji_choose(Game *game, uint32_t index);           // 0부터 시작
char *dongji_input_text(Game *game, const char *utf8_text); // prompt.type == "text"일 때

// 세이브/로드
int32_t dongji_attach_save(Game *game, const char *data); // dongji_start 전에 호출. 성공 1 / 실패 0
char *dongji_save_data(const Game *game);                // 최근 체크포인트, 없으면 NULL

void dongji_string_free(char *s);

#ifdef __cplusplus
}
#endif

#endif
