#include<stdio.h>
extern int vrkaMain(int argc, char** argv);

int main(int argc, char ** argv){
    #ifdef DEBUG
    printf("init\n");
    printf("calling program main\n");
    #endif
    return vrkaMain(argc,argv);
}