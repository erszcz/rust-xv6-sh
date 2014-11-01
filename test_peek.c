#include <stdio.h>
#include <string.h>

char whitespace[] = " \t\r\n\v";

/* Does ltrimmed *ps start with any of toks? */
int
peek(char **ps, char *es, char *toks)
{
  char *s;

  /*fprintf(stderr, "peek: %s toks: %s\n", *ps, toks);*/
  
  s = *ps;
  while(s < es && strchr(whitespace, *s))
    s++;
  *ps = s;
  return *s && strchr(toks, *s);
}

int main(int argc, const char *argv[])
{
    char* s = "   (ala ma kota";
    char* es = s + strlen(s);
    printf("peek(&s, es, \"(\") = %s\n", peek(&s, es, "(") ? "true" : "false");
    printf("peek(&s, es, \"<(\") = %s\n", peek(&s, es, "<(") ? "true" : "false");
    printf("peek(&s, es, \"<\") = %s\n", peek(&s, es, "<") ? "true" : "false");
    return 0;
}
