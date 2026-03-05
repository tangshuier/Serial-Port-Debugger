#include "esp8266.h"

__MESSAGE wifi;
char Str[30];
u8 connected=0;
uint8_t UART_RxFlag;
char UART_RxPacket[128];				//"@MSG\r\n"
int Long;
//底层串口初始化，采用串口2连接ESP8266
void ESP8266_Config(void)
{
	GPIO_InitTypeDef GPIO_InitStruct={0};
	USART_InitTypeDef USART_InitStruct={0};
	NVIC_InitTypeDef NVIC_InitStruct={0};
	
	RCC_APB2PeriphClockCmd(RCC_APB2Periph_GPIOB,ENABLE);
	RCC_APB1PeriphClockCmd(RCC_APB1Periph_USART3,ENABLE);
	
	GPIO_InitStruct.GPIO_Pin = GPIO_Pin_10;//tx
	GPIO_InitStruct.GPIO_Mode = GPIO_Mode_AF_PP;//复用推挽输出
	GPIO_InitStruct.GPIO_Speed = GPIO_Speed_50MHz;
	GPIO_Init(GPIOB,&GPIO_InitStruct);
	
	GPIO_InitStruct.GPIO_Pin = GPIO_Pin_11;//rx
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

	Check_the_network();
	Delay_ms(1000);
}

void USART3_IRQHandler(void)
{
    if (USART_GetITStatus(USART3, USART_IT_RXNE)) {
        
        uint8_t data = USART_ReceiveData(USART3);
        wifi.rxbuff[wifi.rxcount++] = data;
        if (wifi.rxcount >= RXMAX) wifi.rxcount = 0;
    }
    
    if (USART_GetITStatus(USART3, USART_IT_IDLE)) {
        USART3->DR;
        wifi.rxover = 1;
        wifi.rxcount = 0;
    }  
	USART_ClearITPendingBit(USART3, USART_IT_RXNE);
}

//1发送一个字节数据
void USART33_SendByte(uint8_t data)
{
	while(USART_GetFlagStatus(USART3,USART_FLAG_TC)==RESET)
	{
	}
	USART_SendData(USART3,data);
}

void USART3_SendArray(uint8_t *Array, uint16_t Length)
{
	uint16_t i;
	for (i = 0; i < Length; i ++)
	{
		USART33_SendByte(Array[i]);
	}
}
//2发送字符串
void WifiSendStr(char *p)
{
	while(*p !='\0')
	{
		USART33_SendByte(*p++);
	}
}
//3发送一定长度的字符串
void WifiSendbuff(uint8_t *p,uint8_t lenth)
{
	for(uint8_t i=0;i<lenth;i++)
	{
		USART33_SendByte(p[i]);
	}
}

void Check_the_network(void){
	uint8_t buff[512]={0};
	memset(buff, 0, 512);
	sprintf((char*)buff,"%s",CMD_WIFI);		//网络连接测试
	WifiSendStr((char*)buff);
}

void Connect_tothe_cloud(char* x){
	connect_flag = 1;
	uint8_t buff[512]={0};
	memset(buff, 0, 512);
//	sprintf((char*)buff,"%s",x);		//连接到本地服务器
//	WifiSendStr((char*)buff);
//	Delay_ms(1000);
	if(strcmp(x, CMD_CONNECT_BAFA) == 0){
		connected = 1;
		memset(buff, 0, 512);
		sprintf((char*)buff,"%s,%s,%s,%s,%s}",CMD_CONNECT_BAFA,BAFA_PRODUCT_ID,BAFA_TOPIC_CONTROL,BAFA_TOPIC_DATA,BAFA_SERVER_PORT);		//连接到本地服务器
		WifiSendStr((char*)buff);
	}
	else if(strcmp(x, CMD_CONNECT_ALI) == 0){
		connected = 2;
		memset(buff, 0, 512);
		sprintf((char*)buff,"%s,%s,%s,%s,%s}",CMD_CONNECT_ALI,ALI_PRODUCT_KEY,ALI_DEVICE_NAME,ALI_DEVICE_SECRET,ALI_REGION_ID);		//连接到本地服务器
		WifiSendStr((char*)buff);
	}
	else if(strcmp(x, CMD_CONNECT_LOCAL) == 0){
		connected = 3;
		memset(buff, 0, 512);
		sprintf((char*)buff,"%s}",CMD_CONNECT_LOCAL);		//连接到本地服务器
		WifiSendStr((char*)buff);
	}
}

void WIFI_GET_TIME(void){
	uint8_t buff[512]={0};
	memset(buff, 0, 512);
	sprintf((char*)buff,"CMD:GET_TIME}");
	WifiSendStr((char*)buff);
}

extern uint8_t connect_flag;;
void WIFI_FORGET_WIFI(void){
	uint8_t buff[512]={0};
	memset(buff, 0, 512);
	sprintf((char*)buff,"CMD:FORGET_WIFI}");
	WifiSendStr((char*)buff);
	connect_flag = 0;
}

void Get_Weather(void){
	uint8_t buff[128]={0};
	memset(buff, 0, 128);
	sprintf((char*)buff,"%s}",CMD_GET_WEATHER);		
	WifiSendStr((char*)buff);
}

void Set_Weather(char* x){
	uint8_t buff[128]={0};
	memset(buff, 0, 128);
	sprintf((char*)buff,"%s,%s}",CMD_SET_DEFAULT_CITY,x);		
	WifiSendStr((char*)buff);
}

// 将字符转换为数字 (ASCII '0'-'9' 转换为 0-9)
uint8_t charToDigit(char c) {
    if (c >= '0' && c <= '9') {
        return c - '0';
    }
    return 0;
}

// 解析4位数字字符为整数 (例如 "2025" -> 2025)
uint16_t parse4Digit(char* str) {
    return (charToDigit(str[0]) * 1000) + 
           (charToDigit(str[1]) * 100) + 
           (charToDigit(str[2]) * 10) + 
           charToDigit(str[3]);
}

// 解析2位数字字符为整数 (例如 "06" -> 6)
uint8_t parse2Digit(char* str) {
    return (charToDigit(str[0]) * 10) + charToDigit(str[1]);
}

//清空缓存区数据
void Clear_BuffData(void)
{
	memset(wifi.rxbuff,0,RXMAX);
	wifi.rxcount = 0;
	wifi.rxover = 0;
}


// 定义变量类型枚举
typedef enum {
    TYPE_FLOAT,
    TYPE_INT,
    TYPE_UINT8,
	TYPE_UINT16,
	TYPE_UINT32
} VarType;

// 定义通用值联合体
typedef union {
    float *f;
    int *i;
    uint8_t *u8;
	uint16_t *u16;
	uint32_t *u32;
} VarValue;

// 定义结构体来存储名字、类型和值的映射
typedef struct {
    const char *name;
    VarType type;
    VarValue value;
} NameValueMap;

//{"fun":1}
// 定义映射数组
NameValueMap name_flag_maps[] = {
	{"fun",TYPE_FLOAT, {.f = &data.Variable.Humi}},
//    {"Temp_Threshold", TYPE_FLOAT, {.f = &Temp_threshold}},
//	{"Humi_threshold", TYPE_FLOAT, {.f = &Humi_threshold}},
//    {"MQ2_threshold", TYPE_UINT16, {.u16 = &MQ2_Val_threshold}},
//	{"MQ3_threshold", TYPE_UINT16, {.u16 = &MQ3_Val_threshold}},
//	{"SO2_threshold", TYPE_UINT16, {.u16 = &MQ135_Val_threshold}}
    // 可以添加更多映射...
};
#define NUM_MAPS (sizeof(name_flag_maps) / sizeof(name_flag_maps[0]))

char* topic;

char HTTPIP[BUFFER_SIZE];    // 存储提取的IP地址
//*Temp*1*
// DataAnylize：根据 WiFi 数据控制外设
void DataAnylize(void) {
    if (wifi.rxover == 1) {
        wifi.rxover = 0;
        char *topic = NULL;
		if ((topic = strstr((char*)wifi.rxbuff, "Get_Time")) != NULL) {
            // 时间格式: {"Get_Time":"2025-06-16 19:41:36"}
			char *time_start;
			uint16_t offset; // 用于存储偏移量的整数变量
			
			// 找到时间字符串的起始位置（跳过"Get_Time":"）
			time_start = strstr(topic, ":\"");
			if (time_start == NULL) {
				// 格式错误处理
				Clear_BuffData();
				return;
			}
			time_start += 2; // 跳过":
			offset = time_start - (char*)wifi.rxbuff;
			
            // 提取年 (4位)
            MyRTC_Time[0] = parse4Digit((char*)&wifi.rxbuff[offset]);	
                                               
            // 提取月 (2位)                    
            MyRTC_Time[1] = parse2Digit((char*)&wifi.rxbuff[offset+5]);
                                               
            // 提取日 (2位)                    
            MyRTC_Time[2] = parse2Digit((char*)&wifi.rxbuff[offset+8]);
                                               
            // 提取时 (2位)                    
            MyRTC_Time[3] = parse2Digit((char*)&wifi.rxbuff[offset+11]);
                                               
            // 提取分 (2位)                    
            MyRTC_Time[4] = parse2Digit((char*)&wifi.rxbuff[offset+14]);
                                               
            // 提取秒 (2位)                    
            MyRTC_Time[5] = parse2Digit((char*)&wifi.rxbuff[offset+17]);
			
			MyRTC_SetTime();
		}
		if ((topic = strstr((char*)wifi.rxbuff, "WIFI_CONNECTED:")) != NULL) {
			 // 提取冒号后的数字字符并转换为整数
			char* num_start = topic + strlen("WIFI_CONNECTED:");
			connect_flag = atoi(num_start);
		}
		// 新增：解析HTTP地址中的IP
		if ((topic = strstr((char*)wifi.rxbuff, "address:http://")) != NULL) {
				// 定位到IP地址起始位置（跳过"http://"）
				char *ip_start = topic + strlen("address:http://");
				
				// 找到IP地址结束位置（端口号、路径或结束符）
				char *ip_end = strpbrk(ip_start, ":/ \t\n\r");
				if (ip_end == NULL) {
						// 如果没有找到结束符，使用缓冲区末尾
						ip_end = (char*)wifi.rxbuff + strlen((char*)wifi.rxbuff);
				}
				int ip_len = ip_end - ip_start;
				if (ip_len > 0 && ip_len < BUFFER_SIZE) {
						// 提取IP地址
						strncpy(HTTPIP, ip_start, ip_len);
						HTTPIP[ip_len] = '\0';
				}
		}
		//{"Get_Weather":{"city":"Zhengzhou","weather":"中雨","temp":"17.6C"}}
		if ((topic = strstr((char*)wifi.rxbuff, "Get_Weather")) != NULL) {
			// 提取城市信息
			char *city_start = strstr(topic, "city\":\"");
			if (city_start != NULL) {
				city_start += 7; // 跳过 "city":" 
				char *city_end = strstr(city_start, "\"");
				if (city_end != NULL) {
					int city_len = city_end - city_start;
					if (city_len > 0) {
						// 确保不会溢出，最多复制sizeof-1个字符以留出结束符空间
						int copy_len = (city_len < (int)(sizeof(data.Time.city) - 1)) ? city_len : (sizeof(data.Time.city) - 1);
						strncpy(data.Time.city, city_start, copy_len);
						data.Time.city[copy_len] = '\0';
					}
				}
			}
			
			// 提取天气信息
			char *weather_start = strstr(topic, "weather\":\"");
			if (weather_start != NULL) {
				weather_start += 10; // 跳过 "weather":" 
				char *weather_end = strstr(weather_start, "\"");
				if (weather_end != NULL) {
					int weather_len = weather_end - weather_start;
					if (weather_len > 0) {
						// 确保不会溢出，最多复制sizeof-1个字符以留出结束符空间
						int copy_len = (weather_len < (int)(sizeof(data.Time.weather) - 1)) ? weather_len : (sizeof(data.Time.weather) - 1);
						strncpy(data.Time.weather, weather_start, copy_len);
						data.Time.weather[copy_len] = '\0';
					}
				}
			}
			
			// 提取温度信息 - 增加临时缓冲区大小以适应各种长度的温度值
			char *temp_start = strstr(topic, "temp\":\"");
			if (temp_start != NULL) {
				temp_start += 7; // 跳过 "temp":" 
				char *temp_end = strstr(temp_start, "\"");
				if (temp_end != NULL) {
					int temp_len = temp_end - temp_start;
					if (temp_len > 0 && temp_len < 32) { // 限制最大长度以避免缓冲区溢出
						char temp_str[32] = {0}; // 增大临时缓冲区以适应不同长度的温度值
						strncpy(temp_str, temp_start, temp_len);
						data.Time.temp = atof(temp_str); // 转换为浮点数
					}
				}
			}
		}
        // 检查MQTT接收标识
			if(connected == 1)topic = strstr((char*)wifi.rxbuff, BAFA_TOPIC_DATA);
			else if(connected == 2)topic = strstr((char*)wifi.rxbuff, "Message");
			else if(connected == 3) topic = strstr((char*)wifi.rxbuff, "http");
					if (topic != NULL) {
            // 定位到JSON数据起始位置（跳过固定头部）
            char *json_start = strstr(topic, "{");
            if (json_start == NULL) return;  // 没有找到JSON数据
            
            // 遍历映射表，查找并解析每个标签
            for (int i = 0; i < NUM_MAPS; i++) {
				
                // 构造完整的搜索模式："标签":
                char search_pattern[32];
                snprintf(search_pattern, sizeof(search_pattern), "\"%s\":", name_flag_maps[i].name);
                
                // 查找标签位置
                char *tag_pos = strstr(json_start, search_pattern);
                if (tag_pos == NULL) continue;
                
                // 定位到数据起始位置（跳过标签和冒号）
                char *data_start = tag_pos + strlen(search_pattern);
                
                // 找到数据结束位置（逗号、右括号或引号）
                char *data_end = strpbrk(data_start, ",}");
                if (data_end == NULL) continue;
                
                int data_len = data_end - data_start;
                if (data_len <= 0) continue;
                
				
                // 提取数据字符串
                char data_str[32];
                strncpy(data_str, data_start, data_len);
                data_str[data_len] = '\0';
                
                // 根据变量类型解析数据
                switch (name_flag_maps[i].type) {
                    case TYPE_FLOAT:
                        *(name_flag_maps[i].value.f) = (float)atof(data_str);
                        break;
                    case TYPE_INT:
                        *(name_flag_maps[i].value.i) = atoi(data_str);
                        break;
                    case TYPE_UINT8:
                        *(name_flag_maps[i].value.u8) = (uint8_t)atoi(data_str);
                        break;
                    case TYPE_UINT16:
                        *(name_flag_maps[i].value.u16) = (uint16_t)atoi(data_str);
                        break;
                }
            }
        }
		Clear_BuffData();
    }
}
