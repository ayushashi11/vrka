#include <string.h>
#include <stdlib.h>
#include <stdio.h>
char *strrev(char *st){
    char *p1, *p2, *str;
    if(! st || ! *st){
        return str;
    }
    str = (char *)malloc(sizeof(char)*(strlen(st)+1));
    #ifdef DEBUG
    printf("reversing %s",st);
    #endif // 
    
    strcpy(str,st);
    for(p1=str,p2=str+strlen(str)-1; p2>p1; ++p1, --p2){
        *p1^=*p2;
        *p2^=*p1;
        *p1^=*p2;
    }
    return str;
}
char *strrepr(char *st){
    int n = strlen(st);
    int newstrln = 0;
    for(int i=0; i<n; i++){
        if(st[i]=='\n'||st[i]=='\a'||st[i]=='\r'||st[i]=='\b'||st[i]=='\t'){
            newstrln+=2;
        }
        else newstrln+=1;
    }
    char *newstr = (char *)malloc(sizeof(char)*newstrln);
    for(int i=0,j=0;i<newstrln||j<n;j++){
        if(st[j]=='\n'){
            newstr[i]='\\';
            ++i;
            newstr[i]='n';
            ++i;
        }
        else if(st[j]=='\a'){
            newstr[i]='\\';
            ++i;
            newstr[i]='a';
            ++i;
        }
        else if(st[j]=='\r'){
            newstr[i]='\\';
            ++i;
            newstr[i]='r';
            ++i;
        }
        else if(st[j]=='\b'){
            newstr[i]='\\';
            ++i;
            newstr[i]='b';
            ++i;
        }
        else if(st[j]=='\t'){
            newstr[i]='\\';
            ++i;
            newstr[i]='t';
            ++i;
        }
        else{
            newstr[i]=st[j];
            ++i;
        }
    }
    return newstr;
}