#include <stdio.h>
#include <string.h>

char whitespace[] = " \t\r\n\v";
char symbols[] = "<|>&;()";
    
int
gettoken(char **ps, char *es, char **q, char **eq)
{
  char *s;
  int ret;

  /*fprintf(stderr, "gettoken: %s q: %s\n", *ps, *q);*/

  s = *ps;
  while(s < es && strchr(whitespace, *s))
    s++;
  if(q)
    *q = s;
  ret = *s;
  switch(*s){
  case 0:
    break;
  case '|':
  case '(':
  case ')':
  case ';':
  case '&':
  case '<':
    s++;
    break;
  case '>':
    s++;
    if(*s == '>'){
      ret = '+';
      s++;
    }
    break;
  default:
    ret = 'a';
    while(s < es && !strchr(whitespace, *s) && !strchr(symbols, *s))
      s++;
    break;
  }
  if(eq)
    *eq = s;
  
  while(s < es && strchr(whitespace, *s))
    s++;
  *ps = s;
  return ret;
}

void gettoken_simple_command_test() {
    char* s = strdup("/bin/echo a");
    char* es = s + strlen(s);
    char* t;
    char* et;
    int tok;
    int end;
    char bak;

    tok = gettoken(&s, es, &t, &et);
    /*end = et - t;*/
    /*bak = t[end];*/
    /*t[end] = '\0';*/
    /*printf("kind    : %c\n", tok);*/
    /*printf("parsed  : %s\n", *s ? s : "(empty)");*/
    /*printf("token   : %s\n", t);*/
    /*printf("tok len : %d\n", end);*/
    /*t[end] = bak;*/

    /*printf("\n");*/

    tok = gettoken(&s, es, &t, &et);
    end = et - t;
    bak = t[end];
    t[end] = '\0';
    printf("kind    : %c\n", tok);
    printf("parsed  : %s\n", *s ? s : "(empty)");
    printf("token   : %s\n", t);
    /*printf("%s\n", et);*/
    printf("tok len : %d\n", end);
}

void gettoken_lredir_test() {
    char* s = strdup("/bin/echo < a");
    char* es = s + strlen(s);
    char* t;
    char* et;
    int tok;
    int end;
    char bak;

    tok = gettoken(&s, es, &t, &et);
    end = et - t;
    bak = t[end];
    t[end] = '\0';
    printf("kind    : %c\n", tok);
    printf("parsed  : %s\n", *s ? s : "(empty)");
    printf("token   : %s\n", t);
    /*printf("%s\n", et);*/
    printf("tok len : %d\n", end);
    t[end] = bak;

    printf("\n");

    tok = gettoken(&s, es, &t, &et);
    end = et - t;
    bak = t[end];
    t[end] = '\0';
    printf("kind    : %c\n", tok);
    printf("parsed  : %s\n", *s ? s : "(empty)");
    printf("token   : %s\n", t);
    /*printf("%s\n", et);*/
    printf("tok len : %d\n", end);
}

void gettoken_rredir_test() {
    char* s = strdup("/bin/echo > a");
    char* es = s + strlen(s);
    char* t;
    char* et;
    int tok;
    int end;
    char bak;

    tok = gettoken(&s, es, &t, &et);
    end = et - t;
    bak = t[end];
    t[end] = '\0';
    printf("kind    : %c\n", tok);
    printf("parsed  : %s\n", *s ? s : "(empty)");
    printf("token   : %s\n", t);
    /*printf("%s\n", et);*/
    printf("tok len : %d\n", end);
    t[end] = bak;

    printf("\n");

    tok = gettoken(&s, es, &t, &et);
    end = et - t;
    bak = t[end];
    t[end] = '\0';
    printf("kind    : %c\n", tok);
    printf("parsed  : %s\n", *s ? s : "(empty)");
    printf("token   : %s\n", t);
    /*printf("%s\n", et);*/
    printf("tok len : %d\n", end);
}

int main(int argc, const char *argv[])
{
    /*gettoken_lredir_test();*/
    gettoken_rredir_test();
    return 0;
}
