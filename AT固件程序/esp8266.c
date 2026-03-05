#include "esp8266.h"
#include "string.h"
#include "Delay.h"

__MESSAGE wifi={0};
//底层串口初始化，采用串口2连接ESP8266
void ESP8266_Config(void)
{
	GPIO_InitTypeDef GPIO_InitStruct={0};
	USART_InitTypeDef USART_InitStruct={0};
	NVIC_InitTypeDef NVIC_InitStruct={0};
	
	RCC_APB2PeriphClockCmd(RCC_APB2Periph_GPIOB,ENABLE);
	RCC_APB1PeriphClockCmd(RCC_APB1Periph_USART3,ENABLE);
	
	GPIO_InitStruct.GPIO_Pin = GPIO_Pin_10;
	GPIO_InitStruct.GPIO_Mode = GPIO_Mode_AF_PP;//复用推挽输出
	GPIO_InitStruct.GPIO_Speed = GPIO_Speed_50MHz;
	GPIO_Init(GPIOB,&GPIO_InitStruct);
	
	GPIO_InitStruct.GPIO_Pin = GPIO_Pin_11;
	GPIO_InitStruct.GPIO_Mode = GPIO_Mode_IN_FLOATING;//浮空输入
	GPIO_Init(GPIOB,&GPIO_InitStruct);
	
	USART_InitStruct.USART_BaudRate = 115200;
	USART_InitStruct.USART_WordLength = USART_WordLength_8b;
	USART_InitStruct.USART_Parity = USART_Parity_No;
	USART_InitStruct.USART_StopBits = USART_StopBits_1;
	USART_InitStruct.USART_Mode = USART_Mode_Rx |USART_Mode_Tx;
	USART_InitStruct.USART_HardwareFlowControl =USART_HardwareFlowControl_None;
	USART_Init(USART3,&USART_InitStruct);	
	
	USART_ITConfig(USART3,USART_IT_RXNE,ENABLE);//使能USART3接收中断
	USART_ITConfig(USART3,USART_IT_IDLE,ENABLE);//使能USART3接收中断
	
	
	NVIC_InitStruct.NVIC_IRQChannel = USART3_IRQn;
	NVIC_InitStruct.NVIC_IRQChannelCmd = ENABLE;
	NVIC_InitStruct.NVIC_IRQChannelPreemptionPriority = 0;
	NVIC_InitStruct.NVIC_IRQChannelSubPriority = 1;
	NVIC_Init(&NVIC_InitStruct);
	
	USART_Cmd(USART3,ENABLE);	
	Delay_ms(1000);
}



void USART3_IRQHandler(void)
{
	uint8_t data =0;
	if(USART_GetITStatus(USART3,USART_IT_RXNE))
	{
		USART_ClearITPendingBit(USART3,USART_IT_RXNE);
		wifi.rxbuff[wifi.rxcount++] = USART_ReceiveData(USART3);
		data = USART_ReceiveData(USART3); //接收wifi模块回应的数据
	}
	if(USART_GetITStatus(USART3,USART_IT_IDLE))
	{
		USART3->DR;
		wifi.rxover = 1;
		wifi.rxcount =0;
	}
	DataAnylize();
}

//1发送一个字节数据
void Usart33_SendByte(uint8_t data)
{
	while(USART_GetFlagStatus(USART3,USART_FLAG_TC)==RESET)
	{
	}
	USART_SendData(USART3,data);
}

//2发送字符串
void WifiSendStr(char *p)
{
	while(*p !='\0')
	{
		Usart33_SendByte(*p++);
	}
}
//3发送一定长度的字符串
void WifiSendbuff(uint8_t *p,uint8_t lenth)
{
	for(uint8_t i=0;i<lenth;i++)
	{
		Usart33_SendByte(p[i]);
	}
}


//清空缓存区数据
void Clear_BuffData(void)
{
	memset(wifi.rxbuff,0,RXMAX);
	wifi.rxcount = 0;
	wifi.rxover = 0;
}

/**
 * 功能：查找字符串中是否包含另一个字符串
 * 参数：
 *       dest：待查找目标字符串
 *       src：待查找内容
 *       timeout: 查询超时时间
 * 返回值：查找结果  返回所查找字符串在整体字符串中的位置
 *						 = NULL  没有找到字符串		查找失败
 */
char *FindStr(char *dest,char *src,uint32_t outtime)
{
	while((outtime--) && (strstr(dest,src)==NULL))
	{
		Delay_ms(1);
	}
	return strstr(dest,src);
}

/**
 * 功能：ESP8266发送指令获取应答
 * 参数：
 *       cmd -- 指令字符串
 *       ack -- 应答字符串
 *       timeout -- 应答溢出时长
 *			 check_cnt -- 循环发送指令的次数（可能模组没反应过来，多发几次）
 * 返回值：0 -- 成功		>0 -- 失败
 */
uint8_t WifiSendRevAck(char *cmd,char *ack,uint32_t timeout,uint8_t check_cnt)
{
	uint16_t lenth = strlen((char *)cmd);
	wifi.rxcount = 0;
	memset(wifi.rxbuff,0,sizeof(wifi.rxbuff));
	do{
			if(*cmd)  //判断指令是否为空 -- 无效指令
			{ 
				WifiSendbuff((u8 *)cmd,lenth);
			}
			if(*ack) //判断是否有应答 -- 参数是否正确
			{
				if(FindStr((char *)wifi.rxbuff,(char *)ack,timeout)!=NULL)
				{
					wifi.rxcount = 0;
					return 0;
				}
			}
	}while(--check_cnt);
	return 1;
}


//打开透传
uint8_t Wifi_OpenTransmission(void)
{
	return WifiSendRevAck("AT+CIPMODE=1\r\n","OK",1000,2);
}
//关闭透传
void Wifi_CloseTransmission(void)
{
	WifiSendStr("+++");
	Delay_ms(2000);
}


//连接热点
uint8_t ConnectToHotspot(void)
{
	static uint8_t stacnt = 3;
	char buff[36]={0};
	
	Wifi_CloseTransmission();
	
	if(WifiSendRevAck("AT\r\n","OK",100,2)!=0) return 1;
	if(WifiSendRevAck("AT+CWMODE_CUR=1\r\n","OK",100,2)!=0) return 2;
	sprintf(buff,"AT+CWJAP=\"%s\",\"%s\"\r\n",ACCOUNT,PASSWORD);
	if(WifiSendRevAck(buff,"OK",20000,2)!=0) return 3;
	return 0;
}

//连接服务器
uint8_t Wifi_ConnectServer(char *mode,char *ip,uint16_t port)
{
	
	char buff[128]={0};
	sprintf(buff,"AT+CIPSTART=\"%s\",\"%s\",%d\r\n",mode,ip,port);

	if(WifiSendRevAck(buff,"CONNECT",10000,2)==WIFI_ACK_OK)
	{

	}
	else
	{
		return WIFI_ACK_ERROR;
	}
	//设置透传
	if(Wifi_OpenTransmission()==WIFI_ACK_OK)
	{

	}
	else
	{
		return WIFI_ACK_ERROR;
	}
	if(WifiSendRevAck("AT+CIPSEND\r\n",">",500,2)==WIFI_ACK_OK)
	{

	}
	else
	{
		return WIFI_ACK_ERROR;
	}
	return WIFI_ACK_OK;
}


/*******************************************************************************
函数名称：ConnectToOneNetServer
函数作用：链接巴法服务器
函数入口：无
函数出口：无

*******************************************************************************/
uint8_t ConnectToBaffServer(void)
{
	uint8_t Timers = 2;
		
	Wifi_CloseTransmission();    //多次连接需退出透传
	Delay_ms(500);
	
	//连接服务器
	while(Timers--)
	{       
		if(Wifi_ConnectServer("TCP",SERVERIP,SERVERPORT) == WIFI_ACK_OK)
		{
			return 1;
		}
	}
	return 0;
}

#include "stdlib.h"
#include "LED.h"
#include <ctype.h>    // 用于isdigit校验
#include "Motor.h"					//直流电机头文件

char* topic;
extern uint8_t MQ2_Val_threshold;
extern uint8_t Humi1_threshold;
extern int8_t speed;

// DataAnylize：根据 WiFi 数据控制外设
void DataAnylize(void) {
    if (wifi.rxover == 1) {
        wifi.rxover = 0;

        // 如果收到特定命令，直接返回
        if (strstr((char*)wifi.rxbuff, "cmd=2&res=1") != NULL) {
            return;
        }

        // 处理 温湿度阈值 相关逻辑
        if ((topic = strstr((char*)wifi.rxbuff, "333")) != NULL) {
            topic = strstr(topic, "=");
            if (topic != NULL) {
                topic++;
                Humi1_threshold = atoi(topic); // 0-9
//                wifi_control_active = 0; // 标记 WiFi 控制处于活动状态
            }
        }
        // 处理 烟雾浓度阈值 相关逻辑
        if ((topic = strstr((char*)wifi.rxbuff, "444")) != NULL) {
            topic = strstr(topic, "=");
            if (topic != NULL) {
                topic++;
                MQ2_Val_threshold = atoi(topic); // 0-9
//                wifi_control_active = 0; // 标记 WiFi 控制处于活动状态
            }
		}
			        // 处理 风扇 相关逻辑
        if ((topic = strstr((char*)wifi.rxbuff, "555")) != NULL) {
            topic = strstr(topic, "=");
            if (topic != NULL) {
                topic++;
                speed = atoi(topic); // 0-9
				Motor_SetSpeed(speed);
				
//                wifi_control_active = 0; // 标记 WiFi 控制处于活动状态
            }
//            Delay_ms(1000);
            Clear_BuffData();
        }
				
    } 
}


