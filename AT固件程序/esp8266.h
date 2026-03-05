#ifndef __ESP8266_H
#define __ESP8266_H

#include "stm32f10x.h"

//wifi连接参数
#define ACCOUNT  "baimao"    //wifi名称
#define PASSWORD "baimaobaimao" //wifi密码 
#define PRODUCTID   "e655ac9dec7a5f9bfa94bb0a0177f57f"  //密钥
#define SERVERIP    "bemfa.com"
#define SERVERPORT  8344

enum{
WIFI_ACK_OK,WIFI_ACK_ERROR
};

#define RXMAX  1024

typedef struct{
	uint8_t rxbuff[RXMAX];
	uint16_t rxcount;  
	uint8_t rxover;   
	uint8_t txbuff[RXMAX];
}__MESSAGE;

void ESP8266_Config(void);
void Usart2_SendByte(uint8_t data);
void WifiSendStr(char *p);
void WifiSendbuff(uint8_t *p,uint8_t lenth);
void Clear_BuffData(void);
char *FindStr(char *dest,char *src,uint32_t outtime);
uint8_t WifiSendRevAck(char *cmd,char *ack,uint32_t timeout,uint8_t check_cnt);
uint8_t Wifi_OpenTransmission(void);
void Wifi_CloseTransmission(void);
uint8_t ConnectToHotspot(void);
uint8_t Wifi_ConnectServer(char *mode,char *ip,uint16_t port);
uint8_t ConnectToBaffServer(void);

void DataAnylize(void);

#endif

